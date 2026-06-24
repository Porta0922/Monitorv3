param(
    [switch]$NoPause
)

$Host.UI.RawUI.WindowTitle = "ActivityMonitor Agent v3 - Instalador USB"

function Write-Step { param($Num, $Text) Write-Host "`n[$Num/7] $Text" -ForegroundColor Yellow }
function Write-Info { param($Text) Write-Host "  [*] $Text" -ForegroundColor Cyan }
function Write-Ok   { param($Text) Write-Host "  [+] $Text" -ForegroundColor Green }
function Write-Warn { param($Text) Write-Host "  [!] $Text" -ForegroundColor Magenta }
function Write-Err  { param($Text) Write-Host "  [-]" -NoNewline; Write-Host " $Text" -ForegroundColor Red }

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  ActivityMonitor Enterprise Agent v3 - Instalador USB" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""

# ---- Auto-elevate to admin ----
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Info "Solicitando elevacion a Administrador..."
    Start-Process powershell -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit 0
}
Write-Ok "Ejecutando como Administrador"

# ---- Paths ----
$ScriptDir = Split-Path -Parent $PSCommandPath
$ConfigDir = "$env:ProgramData\ActivityMonitor"
$BinDir = "$ConfigDir\Bin"
$DataDir = "$ConfigDir\Data"
$LogDir = "$ConfigDir\logs"
$AgentBin = "$BinDir\activity-monitor-agent.exe"
$EnvFile = "$ConfigDir\.env"
$ServiceName = "ActivityMonitor"
$TaskName = "ActivityMonitorUserAgent"

# ---- Prerequisites ----
Write-Step "0/7" "Verificando requisitos..."

$AgentSource = "$ScriptDir\activity-monitor-agent.exe"
if (-not (Test-Path $AgentSource)) {
    Write-Err "No se encontro activity-monitor-agent.exe junto al script"
    Write-Info "Ejecuta primero: .\scripts\build-usb.ps1"
    if (-not $NoPause) { Read-Host "`nPresiona Enter para salir..." }
    exit 1
}
Write-Ok "Binario encontrado: $AgentSource"

$ConfigSource = "$ScriptDir\agent-config.json"
$ConfigLoaded = $false

# ---- Load configuration ----
$AuthToken = "change-me-in-production"
$OfflineCacheKey = "replace-with-32-byte-cache-key"
$ServerUrl = "http://10.30.0.123:3000"
$RabbitMqUrl = "amqp://eclub:eCLUB123@10.30.0.123:5672/%2f"
$GitHubToken = ""

if (Test-Path $ConfigSource) {
    Write-Info "Cargando configuracion desde agent-config.json..."
    try {
        $config = Get-Content $ConfigSource -Raw | ConvertFrom-Json
        if ($config.agent.authToken) { $AuthToken = $config.agent.authToken }
        if ($config.agent.offlineCacheKey) { $OfflineCacheKey = $config.agent.offlineCacheKey }
        if ($config.agent.githubToken) { $GitHubToken = $config.agent.githubToken }
        if ($config.server.url) { $ServerUrl = $config.server.url }
        if ($config.rabbitmq.url) { $RabbitMqUrl = $config.rabbitmq.url }
        $ConfigLoaded = $true
        Write-Ok "Configuracion cargada desde agent-config.json"
    } catch {
        Write-Warn "Error al leer agent-config.json: $_"
        Write-Info "Usando valores por defecto"
    }
} else {
    Write-Warn "No se encontro agent-config.json, usando valores por defecto"
}

Write-Host ""
Write-Host "  Configuracion:" -ForegroundColor White
Write-Host "    Servidor:   $ServerUrl" -ForegroundColor Gray
Write-Host "    RabbitMQ:   $RabbitMqUrl" -ForegroundColor Gray
Write-Host "    Auth Token: $($AuthToken.Substring(0, [Math]::Min(8, $AuthToken.Length)) + '***')" -ForegroundColor Gray
Write-Host ""

# ---- Installation steps ----
$Progress = 0

# 1. Create directories
Write-Step "1/7" "Creando directorios..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Creando directorios..." -PercentComplete (($Progress / 7) * 100)
@($ConfigDir, $BinDir, $DataDir, $LogDir) | ForEach-Object {
    if (-not (Test-Path $_)) {
        New-Item -ItemType Directory -Path $_ -Force | Out-Null
        Write-Info "Creado: $_"
    }
}
# Grant read/write to Users group
try {
    icacls $ConfigDir /grant:r "*S-1-5-32-545:(OI)(CI)M" /T 2>$null
    Write-Ok "Permisos asignados a Users"
} catch { Write-Warn "No se pudieron asignar permisos: $_" }
Write-Ok "Directorios listos"

# 2. Write .env
Write-Step "2/7" "Escribiendo configuracion..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Escribiendo .env..." -PercentComplete (($Progress / 7) * 100)
@"
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AuthToken
AGENT_OFFLINE_CACHE_KEY=$OfflineCacheKey
AGENT_SERVER_URL=$ServerUrl
RABBITMQ_URL=$RabbitMqUrl
GITHUB_TOKEN=$GitHubToken
"@ | Set-Content -Path $EnvFile -Encoding ascii
Write-Ok "Configuracion guardada en $EnvFile"

# Verify .env content
Write-Info "Verificando .env..."
Get-Content $EnvFile | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }

# 3. Stop old agents
Write-Step "3/7" "Deteniendo agentes previos..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Deteniendo agentes..." -PercentComplete (($Progress / 7) * 100)
Stop-Process -Name "activity-monitor-agent" -Force -ErrorAction SilentlyContinue
Write-Info "Procesos detenidos"

$service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($service) {
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    sc.exe delete $ServiceName 2>$null
    Write-Info "Servicio previo eliminado"
}
Write-Ok "Agentes detenidos"

# 4. Copy binary
Write-Step "4/7" "Copiando binario..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Copiando binario..." -PercentComplete (($Progress / 7) * 100)
Copy-Item -Path $AgentSource -Destination $AgentBin -Force
if (-not (Test-Path $AgentBin)) {
    Write-Err "No se pudo copiar el binario a $AgentBin"
    if (-not $NoPause) { Read-Host "`nPresiona Enter para salir..." }
    exit 1
}
Write-Ok "Binario instalado en $AgentBin"

# 5. Register Windows Service
Write-Step "5/7" "Registrando servicio de Windows..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Registrando servicio..." -PercentComplete (($Progress / 7) * 100)
$scResult = sc.exe create $ServiceName binPath= "`"$AgentBin`"" start= delayed-auto displayName= "ActivityMonitor Enterprise Agent" 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Ok "Servicio registrado"
} else {
    $existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($existingService) {
        sc.exe config $ServiceName binPath= "`"$AgentBin`"" start= delayed-auto 2>$null
        Write-Ok "Servicio actualizado"
    } else {
        Write-Warn "No se pudo registrar el servicio: $scResult"
    }
}

# Configure service recovery (restart on failure)
sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/10000/restart/30000 2>$null

# 6. Create user task
Write-Step "6/7" "Creando tarea de usuario..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Creando tarea programada..." -PercentComplete (($Progress / 7) * 100)

$taskXml = @"
<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Date>2026-06-19T12:00:00</Date>
    <Author>ActivityMonitor</Author>
    <Description>ActivityMonitor User Agent - captures user session activity</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <GroupId>S-1-5-32-545</GroupId>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>99</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"$AgentBin"</Command>
    </Exec>
  </Actions>
</Task>
"@

$taskXmlPath = "$env:TEMP\ActivityMonitorTask.xml"
$taskXml | Set-Content -Path $taskXmlPath -Encoding ASCII

schtasks.exe /Delete /TN $TaskName /F 2>$null
$taskResult = schtasks.exe /Create /XML $taskXmlPath /TN $TaskName /F 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Ok "Tarea de usuario creada"
} else {
    Write-Warn "Error creando tarea por XML: $taskResult"
    schtasks.exe /Create /SC ONLOGON /TN $TaskName /TR "`"$AgentBin`"" /F /IT 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Ok "Tarea creada (fallback ONLOGON)"
    } else {
        Write-Warn "No se pudo crear la tarea programada"
    }
}
Remove-Item -Path $taskXmlPath -Force -ErrorAction SilentlyContinue

# 7. Start agent
Write-Step "7/7" "Iniciando agente..."
$Progress++
Write-Progress -Activity "Instalando ActivityMonitor Agent" -Status "Iniciando agente..." -PercentComplete (($Progress / 7) * 100)

Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc -and $svc.Status -eq 'Running') {
    Write-Ok "Servicio ActivityMonitor iniciado"
} else {
    Write-Warn "El servicio no arranco automaticamente. Ver: sc query $ServiceName"
}

schtasks.exe /Run /TN $TaskName 2>$null
Write-Ok "Tarea de usuario ejecutada"

# ---- Diagnostic summary ----
Write-Progress -Activity "Instalacion completada" -Completed

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host "  INSTALACION COMPLETADA" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Write-Host ""
Write-Host "  Resumen:" -ForegroundColor White
Write-Host "    Binario:  $AgentBin" -ForegroundColor Gray
Write-Host "    Config:   $EnvFile" -ForegroundColor Gray
Write-Host "    Servidor: $ServerUrl" -ForegroundColor Gray
Write-Host "    Logs:     $LogDir" -ForegroundColor Gray
Write-Host ""

# Quick checks
Write-Host "  Verificaciones:" -ForegroundColor White

$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
    Write-Host "    [+] Servicio: $($svc.Status)" -ForegroundColor Green
} else {
    Write-Host "    [-] Servicio: No encontrado" -ForegroundColor Red
}

if (Test-Path $EnvFile) {
    Write-Host "    [+] .env:      Presente" -ForegroundColor Green
    $envContent = Get-Content $EnvFile
    if ($envContent -match "RABBITMQ_URL=.*%2f") {
        Write-Host "    [+] RabbitMQ:  URL con vhost correcto (/%2f)" -ForegroundColor Green
    } else {
        Write-Host "    [-] RabbitMQ:  ATENCION - La URL podria no tener el vhost correcto" -ForegroundColor Red
        Write-Host "         Revisa: type $EnvFile" -ForegroundColor Yellow
    }
} else {
    Write-Host "    [-] .env:      No encontrado" -ForegroundColor Red
}

if (Test-Path $AgentBin) {
    Write-Host "    [+] Binario:   Presente" -ForegroundColor Green
    $versionInfo = (Get-Item $AgentBin).VersionInfo
    if ($versionInfo.ProductVersion) {
        Write-Host "    Version:    $($versionInfo.ProductVersion)" -ForegroundColor Gray
    }
} else {
    Write-Host "    [-] Binario:   No encontrado" -ForegroundColor Red
}

Write-Host ""
Write-Host "  Comandos utiles:" -ForegroundColor White
Write-Host "    sc query $ServiceName" -ForegroundColor Gray
Write-Host "    type $EnvFile" -ForegroundColor Gray
Write-Host "    Get-Content $LogDir\agent_service.log.* -Tail 20" -ForegroundColor Gray
Write-Host "    schtasks /Query /TN $TaskName" -ForegroundColor Gray
Write-Host ""

if ($ConfigLoaded) {
    Write-Info "Configuracion cargada desde agent-config.json"
} else {
    Write-Warn "No se uso agent-config.json - se usaron valores por defecto"
}

if (-not $NoPause) {
    Read-Host "Presiona Enter para salir..."
}
