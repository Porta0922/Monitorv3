param(
    [int]$BucketMaxSeconds = 60,
    [int]$FutureSkewSeconds = 120,
    [switch]$DryRun,
    [switch]$NoBackup
)

$ErrorActionPreference = "Stop"

Write-Host "=== Saneamiento seguro de metricas (hoy) ===" -ForegroundColor Cyan
Write-Host "Proyecto: ActivityMonitor-Enterprise-v3" -ForegroundColor DarkGray

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw "Docker no esta disponible en PATH."
}

try {
    docker compose ps postgres | Out-Null
}
catch {
    throw "No se pudo acceder al servicio 'postgres' via docker compose."
}

$sqlPreview = @"
WITH todays AS (
    SELECT *
    FROM input_activity_metrics
    WHERE "timestamp" >= date_trunc('day', NOW())
),
anomalies AS (
    SELECT id,
           CASE
               WHEN active_seconds < 0 OR idle_seconds < 0 THEN 'negative_seconds'
               WHEN (active_seconds + idle_seconds) > ${BucketMaxSeconds} THEN 'bucket_overflow'
               WHEN "timestamp" > (NOW() + (${FutureSkewSeconds} * INTERVAL '1 second')) THEN 'future_timestamp'
               ELSE 'unknown'
           END AS reason
    FROM todays
    WHERE active_seconds < 0
       OR idle_seconds < 0
       OR (active_seconds + idle_seconds) > ${BucketMaxSeconds}
       OR "timestamp" > (NOW() + (${FutureSkewSeconds} * INTERVAL '1 second'))
),
duplicates AS (
    SELECT id,
           'exact_duplicate' AS reason
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                                 PARTITION BY device_id, "timestamp", status, active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count
                 ORDER BY created_at ASC, id ASC
               ) AS rn
        FROM todays
    ) t
    WHERE t.rn > 1
),
target AS (
    SELECT * FROM anomalies
    UNION ALL
    SELECT * FROM duplicates
)
SELECT reason, COUNT(*) AS rows
FROM target
GROUP BY reason
ORDER BY rows DESC;
"@

$sqlApply = @"
BEGIN;

CREATE TABLE IF NOT EXISTS input_activity_metrics_anomaly_backup (
    LIKE input_activity_metrics INCLUDING ALL
);

WITH todays AS (
    SELECT *
    FROM input_activity_metrics
    WHERE "timestamp" >= date_trunc('day', NOW())
),
anomalies AS (
    SELECT id,
           CASE
               WHEN active_seconds < 0 OR idle_seconds < 0 THEN 'negative_seconds'
               WHEN (active_seconds + idle_seconds) > ${BucketMaxSeconds} THEN 'bucket_overflow'
               WHEN "timestamp" > (NOW() + (${FutureSkewSeconds} * INTERVAL '1 second')) THEN 'future_timestamp'
               ELSE 'unknown'
           END AS reason
    FROM todays
    WHERE active_seconds < 0
       OR idle_seconds < 0
       OR (active_seconds + idle_seconds) > ${BucketMaxSeconds}
         OR "timestamp" > (NOW() + (${FutureSkewSeconds} * INTERVAL '1 second'))
),
duplicates AS (
    SELECT id,
           'exact_duplicate' AS reason
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                                 PARTITION BY device_id, "timestamp", status, active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count
                 ORDER BY created_at ASC, id ASC
               ) AS rn
        FROM todays
    ) t
    WHERE t.rn > 1
),
target AS (
    SELECT * FROM anomalies
    UNION ALL
    SELECT * FROM duplicates
)
INSERT INTO input_activity_metrics_anomaly_backup
SELECT iam.*
FROM input_activity_metrics iam
JOIN target t ON t.id = iam.id;

WITH todays AS (
    SELECT *
    FROM input_activity_metrics
    WHERE "timestamp" >= date_trunc('day', NOW())
),
anomalies AS (
    SELECT id
    FROM todays
    WHERE active_seconds < 0
       OR idle_seconds < 0
       OR (active_seconds + idle_seconds) > ${BucketMaxSeconds}
         OR "timestamp" > (NOW() + (${FutureSkewSeconds} * INTERVAL '1 second'))
),
duplicates AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                                 PARTITION BY device_id, "timestamp", status, active_seconds, idle_seconds, keys_count, mouse_moves_count, clicks_count
                 ORDER BY created_at ASC, id ASC
               ) AS rn
        FROM todays
    ) t
    WHERE t.rn > 1
),
target AS (
    SELECT id FROM anomalies
    UNION
    SELECT id FROM duplicates
)
DELETE FROM input_activity_metrics iam
USING target t
WHERE iam.id = t.id;

COMMIT;
"@

Write-Host "" 
Write-Host "[1/2] Preview de filas anomalias a limpiar..." -ForegroundColor Yellow
$sqlPreview | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1

if ($DryRun) {
    Write-Host "DryRun activo: no se realizaron cambios." -ForegroundColor Green
    exit 0
}

if ($NoBackup) {
    Write-Host "NoBackup activo: se eliminaran anomalias sin respaldo adicional." -ForegroundColor Yellow
    $sqlNoBackup = $sqlApply -replace "(?s)CREATE TABLE IF NOT EXISTS input_activity_metrics_anomaly_backup \(.*?\);\s*", "" -replace "(?s)INSERT INTO input_activity_metrics_anomaly_backup.*?;\s*", ""
    $sqlNoBackup | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1
} else {
    Write-Host "[2/2] Aplicando saneamiento con backup..." -ForegroundColor Yellow
    $sqlApply | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1
}

Write-Host "Saneamiento completado." -ForegroundColor Green
