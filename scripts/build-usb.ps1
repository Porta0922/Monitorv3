param(
    [string]$ServerUrl = "http://localhost:3000",
    [string]$RabbitMqUrl = "amqp://guest:guest@localhost:5672/%2f",
    [string]$AuthToken = "change-me-in-production",
    [string]$OfflineCacheKey = "replace-with-32-byte-cache-key!!",
    [string]$OutputDir = "",
    [string]$OsqueryPolicyProfile = "default",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$AgentDir = Join-Path $RepoRoot "agent"

if (-not $OutputDir) {
    $OutputDir = Join-Path $RepoRoot "usb"
}

function Write-Info  { Write-Host "[*] $($args[0])" -ForegroundColor Cyan }
function Write-Ok   { Write-Host "[+] $($args[0])" -ForegroundColor Green }
function Write-Step { Write-Host "`n>>> $($args[0])" -ForegroundColor Yellow }

# Clean output directory
if (Test-Path $OutputDir) {
    Remove-Item -Path $OutputDir -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  ActivityMonitor USB Builder" -ForegroundColor Cyan
Write-Host "  Genera un paquete listo para USB" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""
Write-Info "Output dir: $OutputDir"
Write-Info "Server:     $ServerUrl"
Write-Info "RabbitMQ:   $RabbitMqUrl"
Write-Host ""

# ---- Step 1: Build agent binary ----
Write-Step "Paso 1/4: Compilando agente (release)..."
if (-not $SkipBuild) {
    Push-Location $AgentDir
    try {
        $proc = Start-Process -FilePath "cargo" -ArgumentList "build --release" -NoNewWindow -Wait -PassThru
        if ($proc.ExitCode -ne 0) {
            throw "cargo build falló con exit code $($proc.ExitCode)"
        }
    } finally {
        Pop-Location
    }

    $binaryName = "activity-monitor-agent.exe"
    $binarySource = Join-Path $RepoRoot "target\release\$binaryName"
    if (-not (Test-Path $binarySource)) {
        $binarySource = Join-Path $AgentDir "target\release\$binaryName"
    }

    if (-not (Test-Path $binarySource)) {
        Write-Error "Binario no encontrado en $binarySource"
        exit 1
    }

    Copy-Item -Path $binarySource -Destination (Join-Path $OutputDir $binaryName) -Force
    Write-Ok "Binario copiado a $OutputDir\$binaryName"
} else {
    $binarySource = Join-Path $OutputDir "activity-monitor-agent.exe"
    if (-not (Test-Path $binarySource)) {
        Write-Error "No hay binario en $OutputDir. Compila sin -SkipBuild primero."
        exit 1
    }
    Write-Ok "Usando binario existente"
}

# ---- Step 2: Generate agent-config.json ----
Write-Step "Paso 2/4: Generando configuracion..."
$config = @{
    apiVersion = "v1"
    version = "3.3.5"
    agent = @{
        authToken = $AuthToken
        offlineCacheKey = $OfflineCacheKey
    }
    server = @{
        url = $ServerUrl
    }
    rabbitmq = @{
        url = $RabbitMqUrl
    }
    osquery = @{
        policyProfile = $OsqueryPolicyProfile
    }
}

$configJson = $config | ConvertTo-Json -Depth 10
$configJson | Out-File -FilePath (Join-Path $OutputDir "agent-config.json") -Encoding utf8
Write-Ok "Configuracion generada"

# ---- Step 3: Copy installer scripts ----
Write-Step "Paso 3/4: Copiando instaladores..."
$installerDir = Join-Path $RepoRoot "Instaladores\Windows"

$installBat = Join-Path $installerDir "install.bat"
if (Test-Path $installBat) {
    Copy-Item -Path $installBat -Destination (Join-Path $OutputDir "install.bat") -Force
    Write-Ok "install.bat copiado"
} else {
    Write-Warning "install.bat no encontrado en $installerDir"
}

$silentBat = Join-Path $installerDir "install-windows-silent.bat"
if (Test-Path $silentBat) {
    Copy-Item -Path $silentBat -Destination (Join-Path $OutputDir "install-silent.bat") -Force
    Write-Ok "install-silent.bat copiado"
}

# ---- Step 4: Generate README ----
Write-Step "Paso 4/4: Generando README..."
$readme = @"
=======================================================
  ActivityMonitor Agent - USB Installation v3.3.5
=======================================================

COMO USAR:
  1. Copia toda esta carpeta a un USB
  2. Conecta el USB en la maquina destino
  3. Ejecuta INSTALL.BAT como Administrador
     (o haz doble click y acepta la elevacion de permisos)

  Instalacion silenciosa (remota):
    install-silent.bat

QUE INSTALA:
  - Binary:    C:\ProgramData\ActivityMonitor\Bin\activity-monitor-agent.exe
  - Config:    C:\ProgramData\ActivityMonitor\.env
  - Servicio:  ActivityMonitor (Session 0, inicio automatico)
  - Tarea:     ActivityMonitorUserAgent (se ejecuta al iniciar sesion)
  - Logs:      C:\ProgramData\ActivityMonitor\logs\

CONFIGURACION:
  Para cambiar la configuracion, edita agent-config.json antes de instalar.

  Server:     $ServerUrl
  RabbitMQ:   $RabbitMqUrl
  AuthToken:  $([regex]::Replace($AuthToken, '.', '*'))

DESINSTALAR:
  En la maquina destino, ejecuta como Admin:
    sc stop ActivityMonitor
    sc delete ActivityMonitor
    schtasks /Delete /TN ActivityMonitorUserAgent /F
    rmdir /s /q C:\ProgramData\ActivityMonitor
"@
$readme | Out-File -FilePath (Join-Path $OutputDir "README.txt") -Encoding ascii
Write-Ok "README.txt generado"

# ---- Summary ----
Write-Host ""
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  USB PACKAGE GENERADO" -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Carpeta: $OutputDir" -ForegroundColor White
Write-Host ""
Get-ChildItem -Path $OutputDir | ForEach-Object {
    $size = if ($_.Length -gt 1MB) {
        "$([math]::Round($_.Length / 1MB, 1)) MB"
    } else {
        "$([math]::Round($_.Length / 1KB, 0)) KB"
    }
    Write-Host "  $($_.Name.PadRight(30)) $size" -ForegroundColor Gray
}
Write-Host ""
Write-Host "  Para usar: Copia toda la carpeta a un USB" -ForegroundColor Yellow
Write-Host "  y ejecuta install.bat en la maquina destino." -ForegroundColor Yellow
Write-Host ""
