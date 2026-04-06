param(
    [switch]$TruncateAll,
    [switch]$NoBackup
)

$ErrorActionPreference = "Stop"

Write-Host "=== Limpieza de metricas input_activity_metrics ===" -ForegroundColor Cyan
Write-Host "Proyecto: ActivityMonitor-Enterprise-v3" -ForegroundColor DarkGray

# Ensure we are at repository root for docker compose.
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    throw "Docker no esta disponible en PATH."
}

# Validate compose service exists/running.
try {
    docker compose ps postgres | Out-Null
}
catch {
    throw "No se pudo acceder al servicio 'postgres' via docker compose. Ejecuta esto desde la carpeta del proyecto y levanta los servicios."
}

if ($TruncateAll) {
    Write-Host "Modo: TRUNCATE total (borra todo el historico)" -ForegroundColor Yellow

    $sqlTruncate = @"
BEGIN;

SELECT COUNT(*) AS total_antes FROM input_activity_metrics;
TRUNCATE TABLE input_activity_metrics;
SELECT COUNT(*) AS total_despues FROM input_activity_metrics;

COMMIT;
"@

    $sqlTruncate | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1

    Write-Host "Listo. Se borro todo el historico de input_activity_metrics." -ForegroundColor Green
    exit 0
}

Write-Host "Modo: limpiar solo registros de hoy" -ForegroundColor Green

if ($NoBackup) {
    Write-Host "Backup: deshabilitado (-NoBackup)" -ForegroundColor Yellow

    $sqlTodayNoBackup = @"
BEGIN;

SELECT COUNT(*) AS rows_hoy_antes
FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

DELETE FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

SELECT COUNT(*) AS rows_hoy_despues
FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

COMMIT;
"@

    $sqlTodayNoBackup | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1

    Write-Host "Listo. Se limpiaron los registros de hoy sin backup." -ForegroundColor Green
    exit 0
}

Write-Host "Backup: habilitado (tabla input_activity_metrics_backup)" -ForegroundColor Green

$sqlTodayWithBackup = @"
BEGIN;

SELECT COUNT(*) AS rows_hoy_antes
FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

CREATE TABLE IF NOT EXISTS input_activity_metrics_backup AS
SELECT * FROM input_activity_metrics WHERE 1 = 0;

INSERT INTO input_activity_metrics_backup
SELECT *
FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

DELETE FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

SELECT COUNT(*) AS rows_hoy_despues
FROM input_activity_metrics
WHERE ""timestamp"" >= date_trunc('day', NOW());

COMMIT;
"@

$sqlTodayWithBackup | docker compose exec -T postgres psql -U monitor_user -d activity_monitor -v ON_ERROR_STOP=1

Write-Host "Listo. Se limpiaron los registros de hoy y se hizo backup." -ForegroundColor Green
