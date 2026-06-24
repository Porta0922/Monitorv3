@echo off
REM ActivityMonitor Enterprise v3 - Windows SILENT Installer
REM Requires: activity-monitor-agent.exe (pre-compiled binary) next to this script
REM Config:   agent-config.json or .env.template (auto-detected)

SETLOCAL ENABLEDELAYEDEXPANSION
set SERVICE_NAME=ActivityMonitor
set SCRIPT_DIR=%~dp0
set LOG_FILE=%TEMP%\ActivityMonitor-Install.log

echo [%DATE% %TIME%] Starting ActivityMonitor silent install... > "%LOG_FILE%"

REM Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [ERR] Este script requiere privilegios de Administrador.
    echo [*] Solicitando elevacion de privilegios...
    echo [%DATE% %TIME%] Elevating to admin... >> "%LOG_FILE%"
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process cmd -ArgumentList '/c \"%~f0\"' -Verb RunAs"
    exit /b 1
)

REM ---- Configuration ----
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor\Config
set INSTALL_DIR=%PROGRAMDATA%\ActivityMonitor
set BIN_DIR=%PROGRAMDATA%\ActivityMonitor\Bin
set DATA_DIR=%PROGRAMDATA%\ActivityMonitor\Data
set AGENT_BIN=%BIN_DIR%\activity-monitor-agent.exe
set LOG_DIR=%PROGRAMDATA%\ActivityMonitor\logs
set ENV_FILE=%INSTALL_DIR%\.env

REM ---- Discovery: locate config JSON (network path or local) ----
set CONFIG_SOURCE=
if exist "%SCRIPT_DIR%agent-config.json" (
    set CONFIG_SOURCE=%SCRIPT_DIR%agent-config.json
    echo [*] Config found: local agent-config.json >> "%LOG_FILE%"
) else if exist "%SCRIPT_DIR%..\agent-config.json" (
    set CONFIG_SOURCE=%SCRIPT_DIR%..\agent-config.json
    echo [*] Config found: parent agent-config.json >> "%LOG_FILE%"
) else if exist "%SCRIPT_DIR%.env.template" (
    echo [*] Using .env.template as config >> "%LOG_FILE%"
) else (
    echo [*] No config file found. Using defaults. >> "%LOG_FILE%"
)

REM ---- Load config from JSON (if found) ----
if not "!CONFIG_SOURCE!"=="" (
    powershell -NoProfile -Command "$c = Get-Content '!CONFIG_SOURCE!' | ConvertFrom-Json; if ($c.agent.authToken) { Set-Content -Path (Join-Path $env:TEMP 'am_auth.txt') -Value $c.agent.authToken } if ($c.agent.offlineCacheKey) { Set-Content -Path (Join-Path $env:TEMP 'am_cachekey.txt') -Value $c.agent.offlineCacheKey } if ($c.server.url) { Set-Content -Path (Join-Path $env:TEMP 'am_server.txt') -Value $c.server.url } if ($c.rabbitmq.url) { Set-Content -Path (Join-Path $env:TEMP 'am_rabbit.txt') -Value $c.rabbitmq.url }" >nul 2>&1
    if exist "%TEMP%\am_auth.txt" set /p AGENT_AUTH_TOKEN=<"%TEMP%\am_auth.txt"
    if exist "%TEMP%\am_cachekey.txt" set /p AGENT_OFFLINE_CACHE_KEY=<"%TEMP%\am_cachekey.txt"
    if exist "%TEMP%\am_server.txt" set /p AGENT_SERVER_URL=<"%TEMP%\am_server.txt"
    if exist "%TEMP%\am_rabbit.txt" set /p RABBITMQ_URL=<"%TEMP%\am_rabbit.txt"
    del "%TEMP%\am_auth.txt" "%TEMP%\am_cachekey.txt" "%TEMP%\am_server.txt" "%TEMP%\am_rabbit.txt" 2>nul
    echo [*] Config loaded from: !CONFIG_SOURCE! >> "%LOG_FILE%"
)

REM ---- Config defaults (if not set by discovery) ----
if "%AGENT_AUTH_TOKEN%"=="" set AGENT_AUTH_TOKEN=change-me-in-production
if "%AGENT_OFFLINE_CACHE_KEY%"=="" set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key
if "%AGENT_SERVER_URL%"=="" set AGENT_SERVER_URL=http://10.30.0.123:3000
if "%RABBITMQ_URL%"=="" set RABBITMQ_URL=amqp://eclub:eCLUB123@10.30.0.123:5672/%%2f

REM ---- Locate pre-compiled binary ----
set AGENT_SRC=
if exist "%SCRIPT_DIR%activity-monitor-agent.exe" (
    set AGENT_SRC=%SCRIPT_DIR%activity-monitor-agent.exe
) else if exist "%SCRIPT_DIR%..\activity-monitor-agent.exe" (
    set AGENT_SRC=%SCRIPT_DIR%..\activity-monitor-agent.exe
) else if exist "%SCRIPT_DIR%..\..\target\release\activity-monitor-agent.exe" (
    set AGENT_SRC=%SCRIPT_DIR%..\..\target\release\activity-monitor-agent.exe
) else if exist "%SCRIPT_DIR%..\..\dist\activity-monitor-agent.exe" (
    set AGENT_SRC=%SCRIPT_DIR%..\..\dist\activity-monitor-agent.exe
)

if "%AGENT_SRC%"=="" (
    echo [ERR] No se encontro activity-monitor-agent.exe pre-compilado.
    echo [*] Copia el binario junto a este script o usa: build-release.ps1
    echo [ERR] Binary not found >> "%LOG_FILE%"
    pause
    exit /b 1
)

echo [*] Using binary: !AGENT_SRC! >> "%LOG_FILE%"

REM ---- Installation steps ----
echo [*] Iniciando instalacion desatendida...
echo.

REM 1. Create directories
echo [1/6] Creando directorios...
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%DATA_DIR%" mkdir "%DATA_DIR%"
icacls "%INSTALL_DIR%" /grant:r *S-1-5-32-545:(OI)(CI)M /T >nul 2>&1
echo     [!] Directorios listos >> "%LOG_FILE%"

REM 2. Write .env
echo [2/6] Escribiendo configuracion...
(
    echo # ActivityMonitor Agent Configuration
    echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
    echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
    echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
    echo RABBITMQ_URL=!RABBITMQ_URL!
) > "%ENV_FILE%"
echo     [!] .env escrito en %ENV_FILE% >> "%LOG_FILE%"

REM 3. Stop old agents
echo [3/6] Deteniendo agentes previos...
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1
sc stop ActivityMonitor >nul 2>&1
sc delete ActivityMonitor >nul 2>&1
echo     [!] Agentes previos detenidos >> "%LOG_FILE%"

REM 4. Copy binary
echo [4/6] Copiando binario...
copy /Y "%AGENT_SRC%" "%AGENT_BIN%" >nul 2>&1
if %errorLevel% neq 0 (
    echo [ERR] Error copiando binario a %AGENT_BIN%
    echo [ERR] Copy failed >> "%LOG_FILE%"
    pause
    exit /b 1
)
echo     [!] Binario copiado a %AGENT_BIN% >> "%LOG_FILE%"

REM 5. Register Windows Service (Session 0)
echo [5/6] Registrando servicio...
sc create ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto displayName= "ActivityMonitor Enterprise Agent" >nul 2>&1
if %errorLevel% equ 0 (
    echo     [!] Servicio registrado >> "%LOG_FILE%"
) else (
    sc query ActivityMonitor >nul 2>&1
    if !errorLevel! equ 0 (
        sc config ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto >nul
        echo     [!] Servicio actualizado >> "%LOG_FILE%"
    ) else (
        echo [ERR] Error registrando servicio >> "%LOG_FILE%"
    )
)

REM 6. Register User Task (no XML - direct schtasks command)
echo [6/6] Configurando inicio automatico de usuario...

REM Use direct schtasks command (more reliable, works on all Windows versions)
schtasks /Create /SC ONLOGON /TN "ActivityMonitorUserAgent" /TR "\"%AGENT_BIN%\"" /F /RL HIGHEST >nul 2>&1
set SCHTASK_ERR=!errorLevel!

if !SCHTASK_ERR! equ 0 (
    echo     [!] Tarea de usuario creada >> "%LOG_FILE%"
) else (
    REM Fallback: Use Registry Run key (works on all Windows versions)
    powershell -NoProfile -ExecutionPolicy Bypass -Command "New-ItemProperty -Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'ActivityMonitorUserAgent' -Value '\"%AGENT_BIN%\"' -PropertyType String -Force | Out-Null" 2>nul
    if !errorLevel! equ 0 (
        echo     [!] Entrada de Registro creada como fallback >> "%LOG_FILE%"
    ) else (
        echo [WARN] No se configuro inicio automatico de usuario >> "%LOG_FILE%"
    )
)

REM ---- Start agents ----
sc start ActivityMonitor >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Servicio iniciado >> "%LOG_FILE%"
) else (
    echo [WARN] Error iniciando servicio (puede iniciar solo) >> "%LOG_FILE%"
)

schtasks /Run /TN "ActivityMonitorUserAgent" >nul 2>&1
echo [*] Tarea de usuario ejecutada >> "%LOG_FILE%"

REM ---- Summary ----
echo.
echo [+] INSTALACION COMPLETADA
echo [*] Binario: %AGENT_BIN%
echo [*] Config:  %ENV_FILE%
echo [*] Server:  %AGENT_SERVER_URL%
echo [*] Log:     %LOG_FILE%
echo [%DATE% %TIME%] Installation complete >> "%LOG_FILE%"

timeout /t 3 >nul
exit /b 0
