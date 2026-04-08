param(
    [switch]$SkipBuild,
    [switch]$SkipNpmInstall,
    [switch]$NoBrowser,
    [switch]$KeepDockerRunning
)

$ErrorActionPreference = "Stop"

Write-Host "=== Reinicio Forzado Demo ===" -ForegroundColor Cyan
Write-Host "Proyecto: ActivityMonitor-Enterprise-v3" -ForegroundColor DarkGray

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

function Assert-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "No se encontro '$Name' en PATH."
    }
}

function Stop-ProcessIfRunning {
    param(
        [string]$ImageName,
        [string]$Label
    )

    $procs = Get-Process -Name $ImageName -ErrorAction SilentlyContinue
    if (-not $procs) {
        Write-Host "- ${Label}: no estaba corriendo" -ForegroundColor DarkGray
        return
    }

    foreach ($proc in $procs) {
        try {
            Stop-Process -Id $proc.Id -Force -ErrorAction Stop
            Write-Host "- $Label detenido (PID $($proc.Id))" -ForegroundColor Yellow
        }
        catch {
            Write-Warning "No se pudo detener $Label (PID $($proc.Id)): $($_.Exception.Message)"
        }
    }
}

Assert-Command docker
Assert-Command cargo
Assert-Command npm

$serverDir = Join-Path $root "server"
$agentDir = Join-Path $root "agent"
$dashboardDir = Join-Path $root "dashboard"
$dashboardNodeModules = Join-Path $dashboardDir "node_modules"

Write-Host "[1/6] Deteniendo procesos locales..." -ForegroundColor Yellow
Stop-ProcessIfRunning -ImageName "activity-monitor-server" -Label "Server"
Stop-ProcessIfRunning -ImageName "activity-monitor-agent" -Label "Agent"
Stop-ProcessIfRunning -ImageName "node" -Label "Node/Vite"

try {
    net stop ActivityMonitor | Out-Null
    Write-Host "- Servicio ActivityMonitor detenido" -ForegroundColor Yellow
}
catch {
    Write-Host "- Servicio ActivityMonitor no estaba iniciado o no pudo detenerse" -ForegroundColor DarkGray
}

Write-Host "[2/6] Reiniciando Docker backend..." -ForegroundColor Yellow
if ($KeepDockerRunning) {
    docker compose up -d
}
else {
    docker compose down
    docker compose up -d
}
docker compose ps

if (-not $SkipNpmInstall -and -not (Test-Path $dashboardNodeModules)) {
    Write-Host "[3/6] Instalando dependencias del dashboard..." -ForegroundColor Yellow
    Set-Location $dashboardDir
    npm install
    Set-Location $root
}
else {
    Write-Host "[3/6] Dependencias dashboard: OK" -ForegroundColor Green
}

if (-not $SkipBuild) {
    Write-Host "[4/6] Compilando agent y server en release..." -ForegroundColor Yellow
    Set-Location $agentDir
    cargo build --release
    Set-Location $serverDir
    cargo build --release
    Set-Location $root
}
else {
    Write-Host "[4/6] Compilacion omitida (-SkipBuild)" -ForegroundColor DarkGray
}

$serverCommand = "Set-Location '$serverDir'; cargo run --release"
$agentCommand = "Set-Location '$agentDir'; cargo run --release"
$dashboardCommand = "Set-Location '$dashboardDir'; npm run dev"

Write-Host "[5/6] Abriendo ventanas de ejecucion..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $serverCommand) -WorkingDirectory $serverDir
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $agentCommand) -WorkingDirectory $agentDir
Start-Process powershell -ArgumentList @('-NoExit', '-Command', $dashboardCommand) -WorkingDirectory $dashboardDir

Write-Host "[6/6] Reinicio completo iniciado." -ForegroundColor Green
Write-Host "- Docker backend arriba" -ForegroundColor Gray
Write-Host "- Server relanzado" -ForegroundColor Gray
Write-Host "- Agent relanzado" -ForegroundColor Gray
Write-Host "- Dashboard relanzado" -ForegroundColor Gray

if (-not $NoBrowser) {
    Start-Process "http://localhost:5173"
}

Write-Host "Listo. Si algun proceso no pudo detenerse, ejecuta este script como Administrador." -ForegroundColor Green