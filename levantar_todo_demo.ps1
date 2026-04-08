param(
    [switch]$SkipNpmInstall,
    [switch]$NoBrowser
)

$ErrorActionPreference = "Stop"

Write-Host "=== Levantar Demo Completa ===" -ForegroundColor Cyan
Write-Host "Proyecto: ActivityMonitor-Enterprise-v3" -ForegroundColor DarkGray

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

function Assert-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "No se encontro '$Name' en PATH."
    }
}

Assert-Command docker
Assert-Command cargo
Assert-Command npm

Write-Host "[1/5] Levantando servicios Docker..." -ForegroundColor Yellow
docker compose up -d

Write-Host "[2/5] Verificando contenedores..." -ForegroundColor Yellow
docker compose ps

$dashboardNodeModules = Join-Path $root "dashboard\node_modules"
if (-not $SkipNpmInstall -and -not (Test-Path $dashboardNodeModules)) {
    Write-Host "[3/5] Instalando dependencias del dashboard..." -ForegroundColor Yellow
    Set-Location (Join-Path $root "dashboard")
    npm install
    Set-Location $root
} else {
    Write-Host "[3/5] Dependencias del dashboard: OK" -ForegroundColor Green
}

$serverDir = Join-Path $root "server"
$agentDir = Join-Path $root "agent"
$dashboardDir = Join-Path $root "dashboard"

$serverCommand = "Set-Location '$serverDir'; cargo run --release"
$agentCommand = "Set-Location '$agentDir'; cargo run --release"
$dashboardCommand = "Set-Location '$dashboardDir'; npm run dev"

Write-Host "[4/5] Abriendo ventanas para server, agent y dashboard..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $serverCommand) -WorkingDirectory $serverDir
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $agentCommand) -WorkingDirectory $agentDir
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $dashboardCommand) -WorkingDirectory $dashboardDir

Write-Host "[5/5] Lanzamiento iniciado." -ForegroundColor Green
Write-Host "- Server:   cargo run --release en /server" -ForegroundColor Gray
Write-Host "- Agent:    cargo run --release en /agent" -ForegroundColor Gray
Write-Host "- Dashboard: npm run dev en /dashboard" -ForegroundColor Gray

if (-not $NoBrowser) {
    Write-Host "Abriendo navegador en http://localhost:5173 ..." -ForegroundColor Yellow
    Start-Process "http://localhost:5173"
}

Write-Host "Listo. Si ya habia procesos corriendo, cierralos manualmente antes de relanzar para evitar binarios bloqueados." -ForegroundColor Green