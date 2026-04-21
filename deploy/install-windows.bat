@echo off
REM ActivityMonitor Enterprise v3 - Windows Installer
REM Registers agent as Windows Service using NSSM (Non-Sucking Service Manager)

SETLOCAL ENABLEDELAYEDEXPANSION
set SERVICE_NAME=ActivityMonitor

REM Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [*] Solicitando permisos de Administrador...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process cmd -ArgumentList '/k \"%~f0\"' -Verb RunAs"
    exit /b 0
)

REM Configuration
set AGENT_NAME=ActivityMonitorAgent
set AGENT_VERSION=0.1.0
set SERVICE_NAME=ActivityMonitor
set AGENT_PATH=%~dp0\..\target\release\activity-monitor-agent.exe
set NSSM_PATH=%~dp0\nssm.exe
set NSSM_LOCAL_DIR=%PROGRAMDATA%\ActivityMonitor\bin
set NSSM_LOCAL_PATH=%NSSM_LOCAL_DIR%\nssm.exe
set NSSM_ZIP_URL=https://nssm.cc/download/nssm-2.24-101-g897c7ad.zip
set NSSM_ZIP_PATH=%TEMP%\nssm.zip
set NSSM_EXTRACT_DIR=%TEMP%\nssm-extract
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor
set LOG_DIR=%PROGRAMDATA%\ActivityMonitor\logs
set ENV_FILE=%CONFIG_DIR%\.env
set OSQUERY_VERSION=5.22.1
set OSQUERY_MSI_URL=https://github.com/osquery/osquery/releases/download/%OSQUERY_VERSION%/osquery-%OSQUERY_VERSION%.msi
set OSQUERY_MSI_PATH=%TEMP%\osquery-%OSQUERY_VERSION%.msi
set AGENT_AUTH_TOKEN=dev-agent-token
set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
set AGENT_SERVER_URL=http://10.30.0.123:3000
set RABBITMQ_URL=amqp://eclub:eCLUB123@10.30.0.123:5672/

:MAIN_MENU
cls
echo ========================================
echo ActivityMonitor Enterprise v3 Installer
echo ========================================
echo Rutas de instalacion:
echo - Ejecutable: %AGENT_PATH%
echo - Configuracion: %CONFIG_DIR%
echo - Logs: %LOG_DIR%
echo ========================================
echo.
echo Seleccione una opcion:
echo 1. Instalar (o actualizar) el agente
echo 2. Modificar credenciales (.env) y reiniciar
echo 3. Desinstalar
echo 4. Salir
echo.
set /p MENU_OPTION="Opcion: "

if "%MENU_OPTION%"=="1" goto INSTALL_AGENT
if "%MENU_OPTION%"=="2" goto MODIFY_CREDS
if "%MENU_OPTION%"=="3" goto UNINSTALL_AGENT
if "%MENU_OPTION%"=="4" goto END_SCRIPT
goto MAIN_MENU

:MODIFY_CREDS
echo.
echo === Modificar Credenciales ===
set /p INPUT_AUTH_TOKEN="Enter agent auth token (or press Enter for default dev-agent-token): "
if not "!INPUT_AUTH_TOKEN!"=="" set AGENT_AUTH_TOKEN=!INPUT_AUTH_TOKEN!

set /p INPUT_SERVER_URL="Enter server URL for remote osquery policy (or press Enter for default http://10.30.0.123:3000): "
if not "!INPUT_SERVER_URL!"=="" set AGENT_SERVER_URL=!INPUT_SERVER_URL!

set /p INPUT_RABBITMQ_URL="Enter RabbitMQ URL (or press Enter for default amqp://eclub:eCLUB123@10.30.0.123:5672/): "
if not "!INPUT_RABBITMQ_URL!"=="" set RABBITMQ_URL=!INPUT_RABBITMQ_URL!

if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
echo Creating configuration file...
(
    echo # ActivityMonitor Agent Configuration
    echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
    echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
    echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
    echo RABBITMQ_URL=!RABBITMQ_URL!
) > "%ENV_FILE%"
echo [+] Credenciales actualizadas en: %ENV_FILE%

sc query %SERVICE_NAME% >nul 2>&1
if !errorLevel! equ 0 (
    echo [*] Reiniciando servicio para aplicar cambios...
    net stop %SERVICE_NAME% >nul 2>&1
    
    set NSSM_CMD=
    if exist "%NSSM_PATH%" set NSSM_CMD="%NSSM_PATH%"
    if "!NSSM_CMD!"=="" if exist "%NSSM_LOCAL_PATH%" set NSSM_CMD="%NSSM_LOCAL_PATH%"
    if not "!NSSM_CMD!"=="" (
        !NSSM_CMD! set %SERVICE_NAME% AppEnvironmentExtra "AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!" "AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!" "AGENT_SERVER_URL=!AGENT_SERVER_URL!" "RABBITMQ_URL=!RABBITMQ_URL!"
    )
    
    net start %SERVICE_NAME%
    echo [+] Servicio reiniciado.
) else (
    echo [!] El servicio no esta instalado. Instale primero para reiniciar.
)
pause
goto MAIN_MENU

:UNINSTALL_AGENT
echo.
echo === Desinstalando ===
sc query %SERVICE_NAME% >nul 2>&1
if !errorLevel! equ 0 (
    echo [*] Deteniendo servicio...
    net stop %SERVICE_NAME% >nul 2>&1
    
    set NSSM_CMD=
    if exist "%NSSM_PATH%" set NSSM_CMD="%NSSM_PATH%"
    if "!NSSM_CMD!"=="" if exist "%NSSM_LOCAL_PATH%" set NSSM_CMD="%NSSM_LOCAL_PATH%"
    if not "!NSSM_CMD!"=="" (
        echo [*] Removiendo servicio con NSSM...
        !NSSM_CMD! remove %SERVICE_NAME% confirm
    ) else (
        echo [*] Removiendo servicio con sc...
        sc delete %SERVICE_NAME%
    )
) else (
    echo [!] El servicio no esta instalado.
)
echo [*] Removiendo tarea programada (watchdog)...
schtasks /Delete /TN ActivityMonitorGuardian /F >nul 2>&1
echo [+] Desinstalacion completada.
pause
goto MAIN_MENU

:INSTALL_AGENT

REM Ask for optional auth token (used by current agent)
set /p INPUT_AUTH_TOKEN="Enter agent auth token (or press Enter for default dev-agent-token): "
if not "!INPUT_AUTH_TOKEN!"=="" (
    set AGENT_AUTH_TOKEN=!INPUT_AUTH_TOKEN!
)

set /p INPUT_SERVER_URL="Enter server URL for remote osquery policy (or press Enter for default http://localhost:3000): "
if not "!INPUT_SERVER_URL!"=="" (
    set AGENT_SERVER_URL=!INPUT_SERVER_URL!
)

set /p INPUT_RABBITMQ_URL="Enter RabbitMQ URL (or press Enter for default amqp://guest:guest@127.0.0.1:5672/): "
if not "!INPUT_RABBITMQ_URL!"=="" (
    set RABBITMQ_URL=!INPUT_RABBITMQ_URL!
)

REM Create config directory
echo.
echo [Paso 1/5] Preparando directorios y configuracion...
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
echo [+] Created config directory: %CONFIG_DIR%

REM Create .env file with variables used by current agent
if not exist "%ENV_FILE%" (
    echo Creating configuration file...
    (
        echo # ActivityMonitor Agent Configuration
        echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
        echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
        echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
        echo RABBITMQ_URL=!RABBITMQ_URL!
    ) > "%ENV_FILE%"
    echo [+] Created configuration: %ENV_FILE%
) else (
    echo [!] Configuration file already exists
)

REM Stop existing service early so the release binary is not locked during build.
sc query %SERVICE_NAME% >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Stopping existing service before build...
    net stop %SERVICE_NAME% >nul 2>&1
)

REM Build latest release binary automatically.
where cargo >nul 2>&1
if %errorLevel% neq 0 (
    echo [-] cargo was not found in PATH. Install Rust toolchain first.
    pause
    exit /b 1
)

echo.
echo [Paso 2/5] Compilando la ultima version del agente...
pushd "%~dp0\.."
cargo build --release -p activity-monitor-agent
if %errorLevel% neq 0 (
    popd
    echo [-] Failed to build release agent binary.
    pause
    exit /b 1
)
popd

if not exist "%AGENT_PATH%" (
    echo [-] Agent binary not found at %AGENT_PATH% after build.
    pause
    exit /b 1
)
echo [+] Release agent ready: %AGENT_PATH%

REM Install osquery if not present
echo.
echo [Paso 3/5] Verificando dependencias (OSQuery)...
if not exist "C:\Program Files\osquery\osqueryi.exe" (
    echo [*] osquery not found. Installing...

    where choco >nul 2>&1
    if !errorLevel! equ 0 (
        echo [*] Chocolatey detected. Installing osquery from Chocolatey repository...
        choco install osquery -y --no-progress
    ) else (
        echo [!] Chocolatey not found. Falling back to direct MSI download [%OSQUERY_VERSION%]...
        powershell -NoProfile -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri '%OSQUERY_MSI_URL%' -OutFile '%OSQUERY_MSI_PATH%'"
        if not exist "%OSQUERY_MSI_PATH%" (
            echo [-] Failed to download osquery MSI
            pause
            exit /b 1
        )

        msiexec /i "%OSQUERY_MSI_PATH%" /qn /norestart
    )

    if !errorLevel! neq 0 (
        echo [-] osquery installation command failed
        pause
        exit /b 1
    )

    if not exist "C:\Program Files\osquery\osqueryi.exe" (
        echo [-] osquery installation did not produce osqueryi.exe
        pause
        exit /b 1
    )

    echo [+] osquery installed successfully
) else (
    echo [+] osquery already installed
)

REM Resolve/install NSSM (local file, PATH, Chocolatey, then direct download fallback)
echo.
echo [Paso 4/5] Instalando gestor de servicios (NSSM)...
set NSSM_CMD=

if exist "%NSSM_PATH%" set NSSM_CMD="%NSSM_PATH%"
if "!NSSM_CMD!"=="" if exist "%NSSM_LOCAL_PATH%" set NSSM_CMD="%NSSM_LOCAL_PATH%"

if "!NSSM_CMD!"=="" (
    where nssm >nul 2>&1
    if !errorLevel! equ 0 (
        for /f "delims=" %%I in ('where nssm') do (
            set NSSM_CMD="%%I"
            goto :nssm_ready
        )
    )
)

if "!NSSM_CMD!"=="" (
    where choco >nul 2>&1
    if !errorLevel! equ 0 (
        echo [*] Chocolatey detected. Installing NSSM from Chocolatey repository...
        choco install nssm -y --no-progress
        where nssm >nul 2>&1
        if !errorLevel! equ 0 (
            for /f "delims=" %%I in ('where nssm') do (
                set NSSM_CMD="%%I"
                goto :nssm_ready
            )
        )
    ) else (
        echo [!] Chocolatey not found. Skipping repository install for NSSM...
    )
)

if "!NSSM_CMD!"=="" (
    echo [*] Falling back to direct NSSM download...
    if not exist "%NSSM_LOCAL_DIR%" mkdir "%NSSM_LOCAL_DIR%"
    if exist "%NSSM_ZIP_PATH%" del /f /q "%NSSM_ZIP_PATH%" >nul 2>&1
    if exist "%NSSM_EXTRACT_DIR%" rmdir /s /q "%NSSM_EXTRACT_DIR%" >nul 2>&1

    powershell -NoProfile -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri '%NSSM_ZIP_URL%' -OutFile '%NSSM_ZIP_PATH%'"
    if not exist "%NSSM_ZIP_PATH%" (
        echo [-] Failed to download NSSM zip
        pause
        exit /b 1
    )

    powershell -NoProfile -Command "Expand-Archive '%NSSM_ZIP_PATH%' -DestinationPath '%NSSM_EXTRACT_DIR%' -Force"
    if exist "%NSSM_EXTRACT_DIR%\nssm-2.24-101-g897c7ad\win64\nssm.exe" (
        copy /y "%NSSM_EXTRACT_DIR%\nssm-2.24-101-g897c7ad\win64\nssm.exe" "%NSSM_LOCAL_PATH%" >nul
    )

    if exist "%NSSM_LOCAL_PATH%" (
        set NSSM_CMD="%NSSM_LOCAL_PATH%"
    )
)

:nssm_ready
if "!NSSM_CMD!"=="" (
    echo [-] Could not resolve or install NSSM automatically.
    pause
    exit /b 1
)
echo [+] NSSM ready: !NSSM_CMD!

REM Stop and remove existing service
echo.
echo [Paso 5/5] Registrando e iniciando servicio de Windows...
echo [*] Checking for existing service...
sc query %SERVICE_NAME% >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Stopping existing service...
    net stop %SERVICE_NAME% >nul 2>&1
    echo [*] Removing existing service...
    !NSSM_CMD! remove %SERVICE_NAME% confirm
)

REM Install new service
echo [*] Installing service...
!NSSM_CMD! install %SERVICE_NAME% "%AGENT_PATH%"
!NSSM_CMD! set %SERVICE_NAME% AppDirectory "%CONFIG_DIR%"
!NSSM_CMD! set %SERVICE_NAME% AppStdout "%LOG_DIR%\output.log"
!NSSM_CMD! set %SERVICE_NAME% AppStderr "%LOG_DIR%\error.log"
!NSSM_CMD! set %SERVICE_NAME% AppRotateFiles 1
!NSSM_CMD! set %SERVICE_NAME% AppRotateOnline 1
!NSSM_CMD! set %SERVICE_NAME% AppRotateSeconds 86400
!NSSM_CMD! set %SERVICE_NAME% AppRotateBytes 10485760
!NSSM_CMD! set %SERVICE_NAME% Start SERVICE_AUTO_START

REM Harden restart behavior: always restart agent if process exits unexpectedly.
!NSSM_CMD! set %SERVICE_NAME% AppExit Default Restart
!NSSM_CMD! set %SERVICE_NAME% AppRestartDelay 5000
!NSSM_CMD! set %SERVICE_NAME% AppThrottle 1500

REM Set environment variables for service
!NSSM_CMD! set %SERVICE_NAME% AppEnvironmentExtra "AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!" "AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!" "AGENT_SERVER_URL=!AGENT_SERVER_URL!" "RABBITMQ_URL=!RABBITMQ_URL!"

REM Configure Service Control Manager recovery actions as an additional safety net.
sc failure %SERVICE_NAME% reset= 0 actions= restart/5000/restart/5000/restart/5000 >nul 2>&1
sc failureflag %SERVICE_NAME% 1 >nul 2>&1

REM Start service
echo [*] Starting service...
net start %SERVICE_NAME%
if %errorLevel% equ 0 (
    echo [+] Service installed and started successfully!
) else (
    echo [-] Failed to start service. Check logs at %LOG_DIR%
    pause
    exit /b 1
)

REM Guardian watchdog: if someone stops the service, start it again automatically.
echo [*] Installing guardian watchdog task...
schtasks /Create /TN "ActivityMonitorGuardian" /SC MINUTE /MO 1 /RU SYSTEM /RL HIGHEST /F /TR "powershell -NoProfile -WindowStyle Hidden -Command \"try { $s = Get-Service -Name 'ActivityMonitor' -ErrorAction Stop; if ($s.Status -ne 'Running') { Start-Service -Name 'ActivityMonitor' -ErrorAction Stop } } catch {}\"" >nul 2>&1
if %errorLevel% equ 0 (
    echo [+] Guardian watchdog task configured (ActivityMonitorGuardian)
) else (
    echo [!] Warning: Could not configure watchdog task. Service recovery is still enabled.
)

echo.
echo ========================================
echo Instalacion Completada
echo ========================================
echo Resumen de configuracion:
echo - API Remota: %AGENT_SERVER_URL%
echo - RabbitMQ:   %RABBITMQ_URL%
echo - Token:      %AGENT_AUTH_TOKEN%
echo.
echo Rutas:
echo - Servicio:   %SERVICE_NAME%
echo - Ejecutable: %AGENT_PATH%
echo - Config:     %CONFIG_DIR%
echo - Logs:       %LOG_DIR%
echo.
echo To manage the service:
echo   Start:   net start ActivityMonitor
echo   Stop:    net stop ActivityMonitor
echo   Update token/key: Edit %ENV_FILE% and reinstall service
echo   Uninstall: nssm remove ActivityMonitor confirm
echo   Remove watchdog: schtasks /Delete /TN ActivityMonitorGuardian /F
echo.
pause
goto MAIN_MENU

:END_SCRIPT
