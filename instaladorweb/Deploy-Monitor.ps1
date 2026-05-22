# Deploy-Monitor.ps1 - Script de deployment automático para Monitor v3
# 
# Uso:
#   PowerShell -ExecutionPolicy Bypass -Command "& { iex (New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/Porta0922/Monitorv3/main/instaladorweb/Deploy-Monitor.ps1') }"
#
# Variables de entorno:
#   MONITOR_DEPLOY_TOKEN - Token de acceso de GitHub (solo lectura)
#   MONITOR_REPO - Repositorio (default: Porta0922/Monitorv3)

param(
    [string]$Token = $env:MONITOR_DEPLOY_TOKEN,
    [string]$Repo = $env:MONITOR_REPO,
    [string]$InstallDir = "C:\Program Files\Monitor",
    [string]$ConfigDir = "C:\ProgramData\Monitor"
)

$ErrorActionPreference = "Stop"

# Valores por defecto
if (-not $Repo) {
    $Repo = "Porta0922/Monitorv3"
}

$SERVICE_NAME = "Monitor"
$LOG_DIR = Join-Path $ConfigDir "logs"

# Funciones de logging
function Write-Info {
    param([string]$Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠️  $Message" -ForegroundColor Yellow
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "❌ $Message" -ForegroundColor Red
    exit 1
}

# Verificar si se ejecuta como administrador
function Test-Admin {
    $currentUser = New-Object Security.Principal.WindowsPrincipal $([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentUser.IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)
}

if (-not (Test-Admin)) {
    Write-Error-Custom "Este script debe ejecutarse como Administrador"
}

Write-Info "Iniciando deployment para Windows"
Write-Info "Repositorio: $Repo"

# Verificar token
if (-not $Token) {
    Write-Error-Custom "Token no configurado. Usa: `$env:MONITOR_DEPLOY_TOKEN='tu_token_aqui'"
}

# Obtener información de la última release
Write-Info "Descargando información de release..."

$headers = @{
    "Authorization" = "token $Token"
    "Accept" = "application/vnd.github.v3+json"
}

try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $headers -UseBasicParsing
} catch {
    Write-Error-Custom "Error al conectar a la API de GitHub: $($_.Exception.Message)"
}

if ($release.message) {
    Write-Error-Custom "Error de API: $($release.message)"
}

# Buscar el asset para Windows
$asset = $release.assets | Where-Object { $_.name -eq "monitor-windows-x86_64.exe" }

if (-not $asset) {
    Write-Error-Custom "Asset 'monitor-windows-x86_64.exe' no encontrado en la release"
}

Write-Info "Descargando: $($asset.name)"
Write-Info "URL: $($asset.browser_download_url)"

# Crear directorios
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-Success "Directorio de instalación creado"
}

if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    Write-Success "Directorio de configuración creado"
}

if (-not (Test-Path $LOG_DIR)) {
    New-Item -ItemType Directory -Path $LOG_DIR -Force | Out-Null
    Write-Success "Directorio de logs creado"
}

# Descargar binario
$tempFile = Join-Path $env:TEMP "monitor_download.exe"
try {
    Write-Info "Descargando binario..."
    Invoke-WebRequest -Uri $asset.browser_download_url -Headers $headers -OutFile $tempFile -UseBasicParsing
    Write-Success "Binario descargado"
} catch {
    Write-Error-Custom "Error descargando el archivo: $($_.Exception.Message)"
}

# Detener servicio si existe
$service = Get-Service -Name $SERVICE_NAME -ErrorAction SilentlyContinue
if ($service) {
    Write-Warning "Deteniendo servicio existente..."
    Stop-Service -Name $SERVICE_NAME -Force
    Start-Sleep -Seconds 3
    Write-Success "Servicio detenido"
}

# Copiar binario a directorio final
$exePath = Join-Path $InstallDir "monitor.exe"
try {
    Copy-Item $tempFile $exePath -Force
    Remove-Item $tempFile -Force
    Write-Success "Binario instalado: $exePath"
} catch {
    Write-Error-Custom "Error instalando el binario: $($_.Exception.Message)"
}

# Crear archivo de configuración si no existe
$configPath = Join-Path $ConfigDir "monitor.conf"
if (-not (Test-Path $configPath)) {
    Write-Info "Creando archivo de configuración..."
    
    $config = @"
# Monitor v3 Configuration
# Generated: $(Get-Date)
# Repository: $Repo

# Puerto para el servidor web (default: 8080)
PORT=8080

# Nivel de logging (debug, info, warn, error)
LOG_LEVEL=info

# Directorio de logs
LOG_DIR=$LOG_DIR

# Intervalo de monitoreo en segundos
CHECK_INTERVAL=60

# Dirección de escucha (0.0.0.0 para todas las interfaces)
BIND_ADDRESS=0.0.0.0
"@
    
    $config | Out-File $configPath -Encoding UTF8
    Write-Success "Archivo de configuración creado"
} else {
    Write-Warning "El archivo de configuración ya existe, omitiendo..."
}

# Crear o actualizar servicio Windows
if ($service) {
    Write-Info "Actualizando servicio Windows existente..."
    # Eliminar servicio anterior
    Remove-Service -Name $SERVICE_NAME -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

Write-Info "Creando servicio Windows..."
try {
    New-Service -Name $SERVICE_NAME `
        -DisplayName "Monitor v3 Service" `
        -BinaryPathName "`"$exePath`"" `
        -StartupType Automatic `
        -Description "Monitor v3 - Automated Monitoring Service (https://github.com/Porta0922/Monitorv3)" `
        -ErrorAction Stop | Out-Null
    
    Write-Success "Servicio creado correctamente"
} catch {
    Write-Error-Custom "Error creando servicio: $($_.Exception.Message)"
}

# Configurar servicio para usar el archivo de configuración
Write-Info "Configurando variables de entorno del servicio..."
try {
    $regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$SERVICE_NAME"
    
    # Crear o actualizar Environment si no existe
    if (-not (Get-Item -Path "$regPath\Parameters" -ErrorAction SilentlyContinue)) {
        New-Item -Path "$regPath\Parameters" -Force | Out-Null
    }
    
    Set-ItemProperty -Path "$regPath\Parameters" -Name "MONITOR_CONFIG" -Value $configPath -Type String
} catch {
    Write-Warning "No se pudo configurar variables de entorno (continuando): $($_.Exception.Message)"
}

# Iniciar servicio
Write-Info "Iniciando servicio..."
try {
    Start-Service -Name $SERVICE_NAME
    Start-Sleep -Seconds 3
    
    $status = (Get-Service -Name $SERVICE_NAME).Status
    if ($status -eq "Running") {
        Write-Success "Servicio iniciado correctamente (Estado: $status)"
    } else {
        Write-Warning "Estado del servicio: $status"
    }
} catch {
    Write-Error-Custom "Error iniciando servicio: $($_.Exception.Message)"
}

# Mostrar información final
Write-Host ""
Write-Host "=== Información de Deployment ===" -ForegroundColor Magenta
Write-Host "Repositorio: $Repo" -ForegroundColor Cyan
Write-Host "Binario: $exePath" -ForegroundColor Cyan
Write-Host "Configuración: $configPath" -ForegroundColor Cyan
Write-Host "Logs: $LOG_DIR" -ForegroundColor Cyan
Write-Host ""

Write-Info "Comandos útiles:"
Write-Host "  Ver estado: Get-Service -Name Monitor" -ForegroundColor Gray
Write-Host "  Reiniciar: Restart-Service -Name Monitor" -ForegroundColor Gray
Write-Host "  Detener: Stop-Service -Name Monitor" -ForegroundColor Gray
Write-Host "  Ver logs: Get-EventLog -LogName System -Source Monitor" -ForegroundColor Gray
Write-Host ""

Write-Success "✨ ¡Deployment completado exitosamente!"
Write-Host ""
Write-Info "El servicio se iniciará automáticamente en futuros reinicios del sistema"
