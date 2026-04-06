param(
    [string]$Date = (Get-Date -Format "yyyy-MM-dd"),
    [switch]$DryRun,
    [switch]$NoBackup
)

$ErrorActionPreference = "Stop"

Write-Host "=== Limpieza segura de basura (USB + Actividad) ===" -ForegroundColor Cyan
Write-Host "Proyecto: ActivityMonitor-Enterprise-v3" -ForegroundColor DarkGray

# Validate date format
$parsedDate = $null
try {
    $parsedDate = [DateTime]::ParseExact($Date, "yyyy-MM-dd", [System.Globalization.CultureInfo]::InvariantCulture)
}
catch {
    throw "Formato de fecha invalido. Usa yyyy-MM-dd (ejemplo: 2026-04-06)."
}

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
WITH
activity_day AS (
    SELECT *
    FROM activity_logs
    WHERE DATE("timestamp") = DATE '$Date'
),
activity_anomalies AS (
    SELECT id, 'unknown_activity' AS reason
    FROM activity_day
    WHERE (
            app_name IS NULL
            OR TRIM(app_name) = ''
            OR LOWER(TRIM(app_name)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            window_title IS NULL
            OR TRIM(window_title) = ''
            OR LOWER(TRIM(window_title)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )

    UNION ALL

    SELECT id, 'invalid_duration' AS reason
    FROM activity_day
    WHERE COALESCE(duration_seconds, 0) <= 0
),
activity_duplicates AS (
    SELECT id, 'duplicate_activity' AS reason
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(app_name, ''), COALESCE(window_title, ''), COALESCE(duration_seconds, 0)
                                 ORDER BY id ASC
               ) AS rn
        FROM activity_day
    ) t
    WHERE t.rn > 1
),
activity_target AS (
    SELECT * FROM activity_anomalies
    UNION ALL
    SELECT * FROM activity_duplicates
),
usb_day AS (
    SELECT *
    FROM usb_events
    WHERE DATE("timestamp") = DATE '$Date'
),
usb_anomalies AS (
    SELECT id, 'internal_disk_noise' AS reason
    FROM usb_day
    WHERE COALESCE(hardware_id, '') !~* '^USBSTOR'
      AND (
            COALESCE(device_name, '') ~* 'nvme|sata|raid|scsi'
          )

    UNION ALL

    SELECT id, 'missing_identifier' AS reason
    FROM usb_day
    WHERE (
            serial_number IS NULL
            OR TRIM(serial_number) = ''
            OR LOWER(TRIM(serial_number)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            hardware_id IS NULL
            OR TRIM(hardware_id) = ''
            OR LOWER(TRIM(hardware_id)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
),
usb_duplicates AS (
    SELECT id, 'duplicate_usb' AS reason
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(action, ''), COALESCE(hardware_id, ''), COALESCE(device_name, ''), COALESCE(serial_number, '')
                                 ORDER BY id ASC
               ) AS rn
        FROM usb_day
    ) t
    WHERE t.rn > 1
),
usb_target AS (
    SELECT * FROM usb_anomalies
    UNION ALL
    SELECT * FROM usb_duplicates
)
SELECT source, reason, rows
FROM (
    SELECT 'activity_logs' AS source, reason, COUNT(*)::BIGINT AS rows
    FROM activity_target
    GROUP BY reason

    UNION ALL

    SELECT 'usb_events' AS source, reason, COUNT(*)::BIGINT AS rows
    FROM usb_target
    GROUP BY reason
) x
ORDER BY source, rows DESC;
"@

$sqlApplyWithBackup = @"
BEGIN;

CREATE TABLE IF NOT EXISTS activity_logs_garbage_backup (
    LIKE activity_logs INCLUDING ALL
);

CREATE TABLE IF NOT EXISTS usb_events_garbage_backup (
    LIKE usb_events INCLUDING ALL
);

WITH
activity_day AS (
    SELECT *
    FROM activity_logs
    WHERE DATE("timestamp") = DATE '$Date'
),
activity_anomalies AS (
    SELECT id
    FROM activity_day
    WHERE (
            app_name IS NULL
            OR TRIM(app_name) = ''
            OR LOWER(TRIM(app_name)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            window_title IS NULL
            OR TRIM(window_title) = ''
            OR LOWER(TRIM(window_title)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )

    UNION

    SELECT id
    FROM activity_day
    WHERE COALESCE(duration_seconds, 0) <= 0
),
activity_duplicates AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(app_name, ''), COALESCE(window_title, ''), COALESCE(duration_seconds, 0)
                                 ORDER BY id ASC
               ) AS rn
        FROM activity_day
    ) t
    WHERE t.rn > 1
),
activity_target AS (
    SELECT id FROM activity_anomalies
    UNION
    SELECT id FROM activity_duplicates
)
INSERT INTO activity_logs_garbage_backup
SELECT a.*
FROM activity_logs a
JOIN activity_target t ON t.id = a.id;

WITH
usb_day AS (
    SELECT *
    FROM usb_events
    WHERE DATE("timestamp") = DATE '$Date'
),
usb_anomalies AS (
    SELECT id
    FROM usb_day
    WHERE COALESCE(hardware_id, '') !~* '^USBSTOR'
      AND (
            COALESCE(device_name, '') ~* 'nvme|sata|raid|scsi'
          )

    UNION

    SELECT id
    FROM usb_day
    WHERE (
            serial_number IS NULL
            OR TRIM(serial_number) = ''
            OR LOWER(TRIM(serial_number)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            hardware_id IS NULL
            OR TRIM(hardware_id) = ''
            OR LOWER(TRIM(hardware_id)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
),
usb_duplicates AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(action, ''), COALESCE(hardware_id, ''), COALESCE(device_name, ''), COALESCE(serial_number, '')
                                 ORDER BY id ASC
               ) AS rn
        FROM usb_day
    ) t
    WHERE t.rn > 1
),
usb_target AS (
    SELECT id FROM usb_anomalies
    UNION
    SELECT id FROM usb_duplicates
)
INSERT INTO usb_events_garbage_backup
SELECT u.*
FROM usb_events u
JOIN usb_target t ON t.id = u.id;

WITH
activity_day AS (
    SELECT *
    FROM activity_logs
    WHERE DATE("timestamp") = DATE '$Date'
),
activity_anomalies AS (
    SELECT id
    FROM activity_day
    WHERE (
            app_name IS NULL
            OR TRIM(app_name) = ''
            OR LOWER(TRIM(app_name)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            window_title IS NULL
            OR TRIM(window_title) = ''
            OR LOWER(TRIM(window_title)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )

    UNION

    SELECT id
    FROM activity_day
    WHERE COALESCE(duration_seconds, 0) <= 0
),
activity_duplicates AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(app_name, ''), COALESCE(window_title, ''), COALESCE(duration_seconds, 0)
                                 ORDER BY id ASC
               ) AS rn
        FROM activity_day
    ) t
    WHERE t.rn > 1
),
activity_target AS (
    SELECT id FROM activity_anomalies
    UNION
    SELECT id FROM activity_duplicates
)
DELETE FROM activity_logs a
USING activity_target t
WHERE a.id = t.id;

WITH
usb_day AS (
    SELECT *
    FROM usb_events
    WHERE DATE("timestamp") = DATE '$Date'
),
usb_anomalies AS (
    SELECT id
    FROM usb_day
    WHERE COALESCE(hardware_id, '') !~* '^USBSTOR'
      AND (
            COALESCE(device_name, '') ~* 'nvme|sata|raid|scsi'
          )

    UNION

    SELECT id
    FROM usb_day
    WHERE (
            serial_number IS NULL
            OR TRIM(serial_number) = ''
            OR LOWER(TRIM(serial_number)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
      AND (
            hardware_id IS NULL
            OR TRIM(hardware_id) = ''
            OR LOWER(TRIM(hardware_id)) IN ('unknown', 'n/a', '<unknown>', '(unknown)')
          )
),
usb_duplicates AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                 PARTITION BY device_id, "timestamp", COALESCE(action, ''), COALESCE(hardware_id, ''), COALESCE(device_name, ''), COALESCE(serial_number, '')
                                 ORDER BY id ASC
               ) AS rn
        FROM usb_day
    ) t
    WHERE t.rn > 1
),
usb_target AS (
    SELECT id FROM usb_anomalies
    UNION
    SELECT id FROM usb_duplicates
)
DELETE FROM usb_events u
USING usb_target t
WHERE u.id = t.id;

COMMIT;
"@

Write-Host ""
Write-Host "Fecha objetivo: $Date" -ForegroundColor Yellow
Write-Host "[1/2] Preview de basura detectada (solo ese dia)..." -ForegroundColor Yellow
$sqlPreview | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1

if ($DryRun) {
    Write-Host "DryRun activo: no se realizaron cambios." -ForegroundColor Green
    exit 0
}

if ($NoBackup) {
    Write-Host "[2/2] Aplicando limpieza SIN backup..." -ForegroundColor Yellow
    $sqlNoBackup = $sqlApplyWithBackup `
        -replace "(?s)CREATE TABLE IF NOT EXISTS activity_logs_garbage_backup \(.*?\);\s*", "" `
        -replace "(?s)CREATE TABLE IF NOT EXISTS usb_events_garbage_backup \(.*?\);\s*", "" `
        -replace "(?s)INSERT INTO activity_logs_garbage_backup.*?;\s*", "" `
        -replace "(?s)INSERT INTO usb_events_garbage_backup.*?;\s*", ""

    $sqlNoBackup | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1
}
else {
    Write-Host "[2/2] Aplicando limpieza CON backup..." -ForegroundColor Yellow
    $sqlApplyWithBackup | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1
}

Write-Host "Limpieza completada para fecha $Date." -ForegroundColor Green
