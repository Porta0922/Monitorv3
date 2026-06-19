param(
    [string]$Version = "3.3.5",
    [string]$ConfigFile = "",
    [switch]$SkipWix,
    [switch]$SkipBuild,
    [switch]$InstallWix
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $RepoRoot "dist"
$AgentDir = Join-Path $RepoRoot "agent"
$InstallerDir = Join-Path $RepoRoot "Instaladores"

function Write-Info  { Write-Host "[*] $($args[0])" -ForegroundColor Cyan }
function Write-Ok   { Write-Host "[+] $($args[0])" -ForegroundColor Green }
function Write-Warn { Write-Host "[!] $($args[0])" -ForegroundColor Yellow }
function Write-Err  { Write-Host "[-] $($args[0])" -ForegroundColor Red }

function Test-Command($cmdname) {
    return [bool](Get-Command -Name $cmdname -ErrorAction SilentlyContinue)
}

function Build-AgentBinary {
    param([string]$TargetDir)

    Write-Info "Compilando agente (release)..."
    Push-Location $AgentDir
    try {
        $proc = Start-Process -FilePath "cargo" -ArgumentList "build --release" -NoNewWindow -Wait -PassThru
        if ($proc.ExitCode -ne 0) {
            throw "cargo build failed with exit code $($proc.ExitCode)"
        }
    } finally {
        Pop-Location
    }

    $binaryName = if ($IsWindows -or $env:OS -match "Windows") { "activity-monitor-agent.exe" } else { "activity-monitor-agent" }
    $binaryPath = Join-Path $RepoRoot "target\release\$binaryName"
    if (-not (Test-Path $binaryPath)) {
        $binaryPath = Join-Path $AgentDir "target\release\$binaryName"
    }
    if (-not (Test-Path $binaryPath)) {
        throw "Binary not found after build (checked workspace and agent target)"
    }

    $destPath = Join-Path $TargetDir $binaryName
    Copy-Item -Path $binaryPath -Destination $destPath -Force
    Write-Ok "Binario copiado a $destPath"

    # Generar checksum
    $hash = (Get-FileHash -Path $binaryPath -Algorithm SHA256).Hash.ToLower()
    $hash | Out-File -FilePath "$destPath.sha256" -Encoding ascii
    Write-Ok "Checksum SHA256: $hash"

    return @{
        BinaryPath = $destPath
        Sha256 = $hash
    }
}

function Install-WiXToolset {
    if (Test-Command "candle") {
        Write-Ok "WiX Toolset ya está instalado"
        return $true
    }

    if ($SkipWix) {
        Write-Warn "WiX instalación saltada (flag -SkipWix)"
        return $false
    }

    if (-not (Test-Command "choco")) {
        Write-Warn "choco no está instalado. No se puede instalar WiX automáticamente."
        return $false
    }

    if (-not $InstallWix) {
        Write-Warn "WiX Toolset no encontrado. Usa -InstallWix para instalarlo automáticamente via choco."
        Write-Warn "O usa -SkipWix para generar solo el ZIP de deployment."
        return $false
    }

    Write-Info "Instalando WiX Toolset via chocolatey..."
    $proc = Start-Process -FilePath "choco" -ArgumentList "install wixtoolset -y --no-progress" -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Err "Error instalando WiX Toolset"
        return $false
    }

    # Recargar PATH para que candle/light estén disponibles
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

    if (-not (Test-Command "candle")) {
        Write-Err "WiX instalado pero candle no está en PATH. Reabre la terminal."
        return $false
    }

    Write-Ok "WiX Toolset instalado correctamente"
    return $true
}

function Build-MsiPackage {
    param(
        [string]$BinaryPath,
        [string]$Version,
        [hashtable]$Config
    )

    $wixDir = Join-Path $DistDir "wix"
    if (-not (Test-Path $wixDir)) { New-Item -ItemType Directory -Path $wixDir -Force | Out-Null }

    $wxsSource = Join-Path $RepoRoot "Instaladores\Windows\ActivityMonitor.wxs"
    if (-not (Test-Path $wxsSource)) {
        Write-Err "WiX source no encontrado en $wxsSource"
        return $null
    }

    # Copiar binario al directorio wix para que WiX lo empaquete
    $wixBinaryDir = Join-Path $wixDir "Files"
    if (-not (Test-Path $wixBinaryDir)) { New-Item -ItemType Directory -Path $wixBinaryDir -Force | Out-Null }
    Copy-Item -Path $BinaryPath -Destination (Join-Path $wixBinaryDir "activity-monitor-agent.exe") -Force

    # Generar archivo de configuración include con los valores
    $configInclude = Join-Path $wixDir "Config.wxi"
    @"
<?xml version="1.0" encoding="utf-8"?>
<Include>
  <?define ProductVersion = "$Version" ?>
  <?define AgentAuthToken = "$($Config.AGENT_AUTH_TOKEN)" ?>
  <?define AgentOfflineCacheKey = "$($Config.AGENT_OFFLINE_CACHE_KEY)" ?>
  <?define AgentServerUrl = "$($Config.AGENT_SERVER_URL)" ?>
  <?define RabbitMqUrl = "$($Config.RABBITMQ_URL)" ?>
</Include>
"@ | Out-File -FilePath $configInclude -Encoding utf8

    Write-Info "Compilando WiX (candle)..."
    $wixObjDir = Join-Path $wixDir "obj"
    if (-not (Test-Path $wixObjDir)) { New-Item -ItemType Directory -Path $wixObjDir -Force | Out-Null }

    $candleArgs = @(
        "-nologo",
        "-arch", "x64",
        "-dWixDir=$wixDir",
        "-swall",
        "-out", "$wixObjDir\",
        $wxsSource
    )
    $proc = Start-Process -FilePath "candle.exe" -ArgumentList $candleArgs -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Err "candle.exe falló (exit code: $($proc.ExitCode))"
        return $null
    }

    Write-Info "Linkeando MSI (light)..."
    $msiOutput = Join-Path $DistDir "ActivityMonitor-Agent-v$Version.msi"
    $lightArgs = @(
        "-nologo",
        "-swall",
        "-out", $msiOutput,
        "-loc", (Join-Path $RepoRoot "Instaladores\Windows\en-US.wxl"),
        (Join-Path $wixObjDir "ActivityMonitor.wixobj")
    )
    $proc = Start-Process -FilePath "light.exe" -ArgumentList $lightArgs -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        Write-Err "light.exe falló (exit code: $($proc.ExitCode))"
        return $null
    }

    $hash = (Get-FileHash -Path $msiOutput -Algorithm SHA256).Hash.ToLower()
    $hash | Out-File -FilePath "$msiOutput.sha256" -Encoding ascii

    Write-Ok "MSI generado: $msiOutput"
    Write-Ok "SHA256: $hash"

    return @{
        MsiPath = $msiOutput
        Sha256 = $hash
    }
}

function Build-DeploymentZip {
    param(
        [string]$BinaryPath,
        [string]$Version,
        [hashtable]$Config
    )

    $zipDir = Join-Path $DistDir "deployment-v$Version"
    if (-not (Test-Path $zipDir)) { New-Item -ItemType Directory -Path $zipDir -Force | Out-Null }

    $platformDir = Join-Path $zipDir "Windows"
    if (-not (Test-Path $platformDir)) { New-Item -ItemType Directory -Path $platformDir -Force | Out-Null }

    # Copiar binario
    $binaryName = "activity-monitor-agent.exe"
    Copy-Item -Path $BinaryPath -Destination (Join-Path $platformDir $binaryName) -Force

    # Copiar silent installer
    $silentInstaller = Join-Path $InstallerDir "Windows\install-windows-silent.bat"
    if (Test-Path $silentInstaller) {
        Copy-Item -Path $silentInstaller -Destination (Join-Path $platformDir "install.bat") -Force
    }

    # Generar .env template
    $envTemplate = @"
# ActivityMonitor Agent Configuration v$Version
AGENT_AUTH_TOKEN=$($Config.AGENT_AUTH_TOKEN)
AGENT_OFFLINE_CACHE_KEY=$($Config.AGENT_OFFLINE_CACHE_KEY)
AGENT_SERVER_URL=$($Config.AGENT_SERVER_URL)
RABBITMQ_URL=$($Config.RABBITMQ_URL)
"@
    $envTemplate | Out-File -FilePath (Join-Path $platformDir ".env.template") -Encoding ascii

    # Generar JSON config para discovery
    $discoveryConfig = @{
        apiVersion = "v1"
        version = $Version
        agent = @{
            authToken = $Config.AGENT_AUTH_TOKEN
            offlineCacheKey = $Config.AGENT_OFFLINE_CACHE_KEY
        }
        server = @{
            url = $Config.AGENT_SERVER_URL
        }
        rabbitmq = @{
            url = $Config.RABBITMQ_URL
        }
    } | ConvertTo-Json
    $discoveryConfig | Out-File -FilePath (Join-Path $platformDir "agent-config.json") -Encoding utf8

    # Generar README rápido para deployment
    $readme = @"
# ActivityMonitor Agent Deployment v$Version

## Instalación Windows
1. Copia la carpeta Windows/ a la máquina destino
2. Ejecuta como Administrador: install.bat
3. El agente se auto-configura usando agent-config.json

## Configuración
Edita agent-config.json antes de copiar para pre-configurar:
- AGENT_SERVER_URL: URL del servidor
- RABBITMQ_URL: URL de RabbitMQ
- AGENT_AUTH_TOKEN: Token de autenticación

## Instalación Silenciosa
install.bat no requiere interacción si agent-config.json existe.
"@
    $readme | Out-File -FilePath (Join-Path $zipDir "README.txt") -Encoding ascii

    # Copiar Linux/macOS installers también
    foreach ($os in @("Linux", "macOS")) {
        $osSrc = Join-Path $InstallerDir $os
        $osDst = Join-Path $zipDir $os
        if (Test-Path $osSrc) {
            Copy-Item -Path $osSrc -Destination $osDst -Recurse -Force
        }
    }

    # Crear ZIP
    $zipPath = Join-Path $DistDir "ActivityMonitor-Agent-v$Version-deployment.zip"
    if (Test-Path $zipPath) { Remove-Item -Path $zipPath -Force }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($zipDir, $zipPath)

    $hash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
    $hash | Out-File -FilePath "$zipPath.sha256" -Encoding ascii

    Write-Ok "Deployment ZIP generado: $zipPath"
    Write-Ok "SHA256: $hash"

    # Cleanup temp dir
    Remove-Item -Path $zipDir -Recurse -Force

    return @{
        ZipPath = $zipPath
        Sha256 = $hash
    }
}

# ---- MAIN ----

Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host "ActivityMonitor Agent Build & Package v$Version" -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan

if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
}

# 1. Load config
$config = @{
    AGENT_AUTH_TOKEN = "change-me-in-production"
    AGENT_OFFLINE_CACHE_KEY = "replace-with-32-byte-cache-key!!"
    AGENT_SERVER_URL = "http://localhost:3000"
    RABBITMQ_URL = "amqp://guest:guest@localhost:5672/"
    AGENT_OSQUERY_POLICY_PROFILE = "default"
}
if ($ConfigFile -and (Test-Path $ConfigFile)) {
    Write-Info "Cargando configuración desde $ConfigFile"
    $fileConfig = Get-Content $ConfigFile | ConvertFrom-Json
    foreach ($key in $fileConfig.PSObject.Properties.Name) {
        $config[$key] = $fileConfig.$key
    }
}

# 2. Build binary
if (-not $SkipBuild) {
    $binaryInfo = Build-AgentBinary -TargetDir $DistDir
} else {
    $binaryName = "activity-monitor-agent.exe"
    $binaryPath = Join-Path $DistDir $binaryName
    if (-not (Test-Path $binaryPath)) {
        Write-Err "No se encontró binario en $binaryPath. Compila sin -SkipBuild primero."
        exit 1
    }
    $binaryInfo = @{ BinaryPath = $binaryPath }
}

# 3. Try building MSI
$msiBuilt = $false
if (-not $SkipWix) {
    $wixAvailable = Install-WiXToolset
    if ($wixAvailable) {
        $msiResult = Build-MsiPackage -BinaryPath $binaryInfo.BinaryPath -Version $Version -Config $config
        if ($msiResult) {
            $msiBuilt = $true
        }
    }
}

# 4. Fallback: deployment ZIP
$zipResult = Build-DeploymentZip -BinaryPath $binaryInfo.BinaryPath -Version $Version -Config $config

# 5. Summary
Write-Host "`n=========================================================" -ForegroundColor Cyan
Write-Host "BUILD COMPLETE - v$Version" -ForegroundColor Cyan
Write-Host "=========================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output directory: $DistDir" -ForegroundColor White
Get-ChildItem -Path $DistDir | ForEach-Object {
    Write-Host "  $($_.Name) ($( [math]::Round($_.Length / 1MB, 2) ) MB)" -ForegroundColor Gray
}
Write-Host ""
Write-Host "Quick deploy commands:" -ForegroundColor Yellow
Write-Host "  PowerShell: .\scripts\build-release.ps1 -Version $Version" -ForegroundColor Gray
Write-Host "  With config: .\scripts\build-release.ps1 -Version $Version -ConfigFile .\deploy-config.json" -ForegroundColor Gray
Write-Host "  Skip WiX:    .\scripts\build-release.ps1 -Version $Version -SkipWix" -ForegroundColor Gray
