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
set AGENT_PATH=%~dp0\..\target\release\activity-monitor-agent.exe
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor
set BIN_DIR=%PROGRAMDATA%\ActivityMonitor\Bin
set AGENT_BIN=%BIN_DIR%\activity-monitor-agent.exe
set LOG_DIR=%PROGRAMDATA%\ActivityMonitor\logs
set ENV_FILE=%CONFIG_DIR%\.env
set OSQUERY_VERSION=5.22.1
set OSQUERY_MSI_URL=https://github.com/osquery/osquery/releases/download/%OSQUERY_VERSION%/osquery-%OSQUERY_VERSION%.msi
set OSQUERY_MSI_PATH=%TEMP%\osquery-%OSQUERY_VERSION%.msi
set AGENT_AUTH_TOKEN=change-me-in-production
set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
set AGENT_SERVER_URL=http://10.30.0.123:3000
set RABBITMQ_URL=amqp://eclub:eCLUB123@10.30.0.123:5672/%%2f

:MAIN_MENU
cls
echo ========================================
echo ActivityMonitor Enterprise v3.1.0-HYBRID
echo ========================================
echo Rutas de instalacion:
echo - Ejecutable: %AGENT_PATH%
echo - Configuracion: %CONFIG_DIR%
echo - Logs: %LOG_DIR%
echo ========================================
echo.
echo Seleccione una opcion:
echo 1. Instalacion COMPLETA (Servicio + Tarea de Usuario)
echo 2. Instalar solo SERVICIO (Segundo plano/Admin)
echo 3. Instalar solo TAREA DE USUARIO (Captura Actividad/Idle)
echo 4. Modificar credenciales (.env) y reiniciar
echo 5. Actualizar binario (Mantiene configuracion)
echo 6. Desinstalar TODO
echo 7. Salir
echo.
set /p MENU_OPTION="Opcion: "

if "%MENU_OPTION%"=="1" set MODE=FULL & goto INSTALL_AGENT
if "%MENU_OPTION%"=="2" set MODE=SERVICE & goto INSTALL_AGENT
if "%MENU_OPTION%"=="3" set MODE=USER & goto INSTALL_AGENT
if "%MENU_OPTION%"=="4" goto MODIFY_CREDS
if "%MENU_OPTION%"=="5" goto UPDATE_ONLY
if "%MENU_OPTION%"=="6" goto UNINSTALL_AGENT
if "%MENU_OPTION%"=="7" goto END_SCRIPT
goto MAIN_MENU

:MODIFY_CREDS
echo.
echo === Modificar Credenciales ===
set /p INPUT_AUTH_TOKEN="Enter agent auth token (or press Enter for default dev-agent-token): "
if not "!INPUT_AUTH_TOKEN!"=="" set AGENT_AUTH_TOKEN=!INPUT_AUTH_TOKEN!

set /p INPUT_SERVER_URL="Enter server URL for remote osquery policy (or press Enter for default http://10.30.0.123:3000): "
if not "!INPUT_SERVER_URL!"=="" set AGENT_SERVER_URL=!INPUT_SERVER_URL!

set /p INPUT_RABBITMQ_URL="Enter RabbitMQ URL (or press Enter for default amqp://eclub:eCLUB123@10.30.0.123:5672/%%2f): "
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

taskkill /F /IM activity-monitor-agent.exe >nul 2>&1
    
    echo [*] Iniciando agente...
    start "" "%AGENT_PATH%"
    echo [+] Credenciales actualizadas y agente reiniciado.
pause
goto MAIN_MENU

:UNINSTALL_AGENT
echo.
echo [*] Limpiando servicio NSSM antiguo si existe...
sc query ActivityMonitor >nul 2>&1
if !errorLevel! equ 0 (
    net stop ActivityMonitor >nul 2>&1
    sc delete ActivityMonitor >nul 2>&1
)

echo [*] Deteniendo agente en ejecucion...
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1

echo [*] Removiendo de inicio automatico (Registro Run)...
REG DELETE "HKLM\Software\Microsoft\Windows\CurrentVersion\Run" /v ActivityMonitorAgent /f >nul 2>&1

echo [*] Removiendo tarea programada (User Agent)...
schtasks /Delete /TN ActivityMonitorUserAgent /F >nul 2>&1
echo [+] Desinstalacion completada.
pause
goto MAIN_MENU

:UPDATE_ONLY
echo.
echo [*] Iniciando actualizacion rapida (manteniendo configuracion actual)...
set MODE=FULL
goto STEP_1

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

set /p INPUT_RABBITMQ_URL="Enter RabbitMQ URL (or press Enter for default amqp://guest:guest@127.0.0.1:5672/%%2f): "
if not "!INPUT_RABBITMQ_URL!"=="" (
    set RABBITMQ_URL=!INPUT_RABBITMQ_URL!
)

:STEP_1
echo.
echo [1/8] Preparando directorios...
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%DATA_DIR%" mkdir "%DATA_DIR%"
icacls "%INSTALL_DIR%" /grant:r *S-1-5-32-545:(OI)(CI)M /T >nul 2>&1
echo     ^> Directorios y permisos listos en %INSTALL_DIR% [OK]

echo [2/8] Configurando archivo .env...
if not exist "%ENV_FILE%" (
    (
        echo # ActivityMonitor Agent Configuration
        echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
        echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
        echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
        echo RABBITMQ_URL=!RABBITMQ_URL!
    ) > "%ENV_FILE%"
    echo     ^> Archivo .env creado en %ENV_FILE% [OK]
) else (
    echo     ^> Archivo .env ya existe - se mantiene actual [OK]
)

REM Cleanup old service if it exists
sc query ActivityMonitor >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Stopping old service before build...
    net stop ActivityMonitor >nul 2>&1
    sc delete ActivityMonitor >nul 2>&1
)

echo [*] Deteniendo agente existente si esta en ejecucion...
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1

REM Check if a pre-built binary already exists
if exist "%AGENT_PATH%" (
    echo.
    echo [+] Se detecto un binario pre-compilado en: %AGENT_PATH%
    set /p USE_EXISTING="¿Desea usar este binario en lugar de compilar uno nuevo? (S/N) [S]: "
    if "!USE_EXISTING!"=="" set USE_EXISTING=S
    if /i "!USE_EXISTING!"=="S" goto VERIFY_BINARY
)

REM Build latest release binary automatically if cargo is available.
where cargo >nul 2>&1
if %errorLevel% equ 0 goto CARGO_BUILD

echo.
echo [*] cargo no encontrado. Intentando instalar Rust...
echo [*] Descargando rustup-init.exe...
powershell -NoProfile -Command "Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile %TEMP%\rustup-init.exe"
if not exist "%TEMP%\rustup-init.exe" (
    echo [-] No se pudo descargar el instalador de Rust.
    goto CHECK_PREBUILT
)

echo [*] Ejecutando instalador de Rust (esto puede tardar unos minutos)...
echo [*] Se usara la instalacion por defecto (-y).
"%TEMP%\rustup-init.exe" -y --default-toolchain stable
if %errorLevel% neq 0 (
    echo [-] Error al instalar Rust.
    goto CHECK_PREBUILT
)

REM Add cargo to current session PATH
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>&1
if %errorLevel% neq 0 (
    echo [-] Rust se instalo pero cargo no esta en el PATH de esta sesion.
    goto CHECK_PREBUILT
)
echo [+] Rust instalado y configurado correctamente.

:CARGO_BUILD
echo.
echo [Paso 2/5] Compilando la ultima version del agente con cargo...
pushd "%~dp0\.."
cargo build --release -p activity-monitor-agent
if %errorLevel% neq 0 (
    popd
    echo [-] Failed to build release agent binary.
    pause
    exit /b 1
)
popd
goto VERIFY_BINARY

:CHECK_PREBUILT
echo.
echo [Paso 2/5] cargo no encontrado. Verificando binario pre-compilado...
if not exist "%AGENT_PATH%" (
    echo [-] cargo no esta en PATH y no se encontro binario en %AGENT_PATH%.
    echo [-] Por favor compile el agente en otra maquina o instale Rust.
    pause
    exit /b 1
)
echo [+] Usando binario pre-compilado en target\release\.

:VERIFY_BINARY
if not exist "%AGENT_PATH%" (
    echo [-] No se pudo encontrar el binario del agente en %AGENT_PATH%.
    pause
    exit /b 1
)
echo [+] Release agent ready: %AGENT_PATH%

echo [4/8] Desplegando binario en la ruta local...
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
copy /Y "%AGENT_PATH%" "%AGENT_BIN%" >nul
if %errorLevel% neq 0 (
    echo     [-] Error: No se pudo copiar el binario a %AGENT_BIN%
    pause
    exit /b 1
)
echo     ^> Binario copiado a %AGENT_BIN% [OK]

REM Install osquery if not present
echo [5/8] Verificando dependencias (OSQuery)...
if not exist "C:\Program Files\osquery\osqueryi.exe" (
    echo     [*] osquery no encontrado. Iniciando instalacion...
    where choco >nul 2>&1
    if !errorLevel! equ 0 (
        choco install osquery -y --no-progress
    ) else (
        powershell -NoProfile -Command "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri '%OSQUERY_MSI_URL%' -OutFile '%OSQUERY_MSI_PATH%'"
        if not exist "%OSQUERY_MSI_PATH%" (
            echo     [-] Error: No se pudo descargar el instalador de osquery.
            pause
            exit /b 1
        )
        msiexec /i "%OSQUERY_MSI_PATH%" /qn /norestart
    )
    if not exist "C:\Program Files\osquery\osqueryi.exe" (
        echo     [-] Error: osquery no se instalo correctamente.
        pause
        exit /b 1
    )
    echo     ^> osquery instalado y verificado [OK]
) else (
    echo     ^> osquery ya esta instalado [OK]
)

echo [6/8] Registrando Servicio de Windows (Sesion 0)...
if "%MODE%"=="USER" (
    echo     ^> Saltando registro de servicio - Modo Solo Usuario [SKIP]
) else (
    REG DELETE "HKLM\Software\Microsoft\Windows\CurrentVersion\Run" /v ActivityMonitorAgent /f >nul 2>&1
    sc create ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto displayName= "ActivityMonitor Enterprise Agent" >nul
    if %errorLevel% equ 0 (
        echo     ^> Servicio registrado correctamente [OK]
    ) else (
        sc query ActivityMonitor >nul 2>&1
        if !errorLevel! equ 0 (
            sc config ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto >nul
            echo     ^> Servicio actualizado [OK]
        ) else (
            echo     [-] Error: No se pudo registrar el servicio.
            pause
            exit /b 1
        )
    )
)

echo [7/8] Configurando tarea de Sesion de Usuario...
if "%MODE%"=="SERVICE" (
    echo     ^> Saltando registro de tarea de usuario - Modo Solo Servicio [SKIP]
) else (
    schtasks /Create /SC ONLOGON /TN "ActivityMonitorUserAgent" /TR "\"%AGENT_BIN%\"" /F >nul 2>&1
    if %errorLevel% equ 0 (
        echo     ^> Tarea de inicio de sesion creada [OK]
        schtasks /Run /TN "ActivityMonitorUserAgent" >nul 2>&1
        echo     ^> Captura de actividad iniciada [OK]
    )
)

echo [8/8] Iniciando servicios...
if not "%MODE%"=="USER" (
    sc start ActivityMonitor >nul 2>&1
)
echo     ^> Proceso finalizado [OK]

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
echo - Servicio:   Activado (ActivityMonitor)
echo - Ejecutable: %AGENT_BIN%
echo - Config:     %CONFIG_DIR%
echo - Logs:       %LOG_DIR%
echo.
echo To manage the agent:
echo   Update token/key: Option 2 in this installer
echo   Uninstall:        Option 3 in this installer
echo.
pause
goto MAIN_MENU

:END_SCRIPT
exit
