@echo off
REM ActivityMonitor Enterprise v3 - USB Installer
REM Double-click friendly. Auto-elevates to admin.
REM Requires: activity-monitor-agent.exe + agent-config.json (same folder)

SETLOCAL ENABLEDELAYEDEXPANSION
set SCRIPT_DIR=%~dp0
set SERVICE_NAME=ActivityMonitor

REM ---- Handle uninstall flag ----
if /i "%1"=="/uninstall" goto UNINSTALL
if /i "%1"=="/?" goto HELP

REM ---- Auto-elevate to admin ----
net session >nul 2>&1
if %errorLevel% neq 0 (
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process cmd -ArgumentList '/c \"%~f0\"' -Verb RunAs"
    exit /b 0
)

:MAIN
cls
echo ============================================================
echo     ActivityMonitor Enterprise Agent v3 - Instalador USB
echo ============================================================
echo.
echo   Este instalador configura el agente de monitoreo
echo   en segundo plano. No requiere internet ni compilacion.
echo.

REM ---- Verify prerequisites ----
if not exist "%~dp0activity-monitor-agent.exe" (
    echo  [-] ERROR: No se encontro activity-monitor-agent.exe
    echo  [*] Copia el binario compilado junto a este script.
    echo  [*] Ejecuta: .\scripts\build-usb.ps1 para generarlo.
    echo.
    pause
    exit /b 1
)

if not exist "%~dp0agent-config.json" (
    echo  [!] No se encontro agent-config.json
    echo  [*] Se usaran valores por defecto (localhost).
    echo  [*] Para configurar, edita agent-config.json y ejecuta de nuevo.
    echo.
    choice /C SN /M "Continuar con valores por defecto?"
    if errorlevel 2 exit /b 0
)

REM ---- Load configuration ----
set AGENT_AUTH_TOKEN=change-me-in-production
set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
set AGENT_SERVER_URL=http://localhost:3000
set RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2f

if exist "%~dp0agent-config.json" (
    for /f "usebackq delims=" %%a in (`powershell -NoProfile -Command "Get-Content '%~dp0agent-config.json' | ConvertFrom-Json | ForEach-Object { $_.agent.authToken + '|' + $_.agent.offlineCacheKey + '|' + $_.server.url + '|' + $_.rabbitmq.url }"`) do set "CONFIG_LINE=%%a"
    for /f "tokens=1-4 delims=|" %%a in ("!CONFIG_LINE!") do (
        if not "%%a"=="" set AGENT_AUTH_TOKEN=%%a
        if not "%%b"=="" set AGENT_OFFLINE_CACHE_KEY=%%b
        if not "%%c"=="" set AGENT_SERVER_URL=%%c
        if not "%%d"=="" set RABBITMQ_URL=%%d
    )
)

REM ---- Show configuration summary ----
echo  Configuracion:
echo    Servidor:   !AGENT_SERVER_URL!
echo    RabbitMQ:   !RABBITMQ_URL!
echo    Auth Token: !AGENT_AUTH_TOKEN!
echo.
echo  Presiona cualquier tecla para comenzar la instalacion...
pause >nul

cls
echo ============================================================
echo     Instalando ActivityMonitor Agent...
echo ============================================================
echo.

REM ---- Paths ----
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor
set BIN_DIR=%CONFIG_DIR%\Bin
set DATA_DIR=%CONFIG_DIR%\Data
set LOG_DIR=%CONFIG_DIR%\logs
set AGENT_BIN=%BIN_DIR%\activity-monitor-agent.exe
set ENV_FILE=%CONFIG_DIR%\.env

REM ---- 1. Create directories ----
echo  [1/7] Creando directorios...
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%DATA_DIR%" mkdir "%DATA_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
icacls "%CONFIG_DIR%" /grant:r *S-1-5-32-545:(OI)(CI)M /T >nul 2>&1
echo    [+] Directorios listos

REM ---- 2. Write .env ----
echo  [2/7] Escribiendo configuracion...
(
    echo # ActivityMonitor Agent Configuration
    echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
    echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
    echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
    echo RABBITMQ_URL=!RABBITMQ_URL!
) > "%ENV_FILE%"
echo    [+] Configuracion guardada

REM ---- 3. Stop old agents ----
echo  [3/7] Deteniendo agentes previos...
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1
sc stop ActivityMonitor >nul 2>&1
sc delete ActivityMonitor >nul 2>&1
echo    [+] Agentes detenidos

REM ---- 4. Copy binary ----
echo  [4/7] Copiando binario...
copy /Y "%~dp0activity-monitor-agent.exe" "%AGENT_BIN%" >nul
if %errorLevel% neq 0 (
    echo  [-] ERROR: No se pudo copiar el binario
    echo    Destino: %AGENT_BIN%
    pause
    exit /b 1
)
echo    [+] Binario instalado

REM ---- 5. Register Windows Service ----
echo  [5/7] Registrando servicio de Windows...
sc create ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto displayName= "ActivityMonitor Enterprise Agent" >nul 2>&1
if %errorLevel% equ 0 (
    echo    [+] Servicio registrado
) else (
    sc query ActivityMonitor >nul 2>&1
    if !errorLevel! equ 0 (
        sc config ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto >nul
        echo    [+] Servicio actualizado
    ) else (
        echo  [!] No se pudo registrar el servicio
    )
)

REM ---- 6. Create user task ----
echo  [6/7] Creando tarea de usuario...
set "TASK_XML=%TEMP%\ActivityMonitorTask.xml"
(
    echo ^<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task"^>
    echo   ^<RegistrationInfo^>
    echo     ^<Date^>2026-06-19T12:00:00^</Date^>
    echo     ^<Author^>ActivityMonitor^</Author^>
    echo     ^<Description^>ActivityMonitor User Agent^</Description^>
    echo   ^</RegistrationInfo^>
    echo   ^<Triggers^>
    echo     ^<LogonTrigger^>^<Enabled^>true^</Enabled^>^</LogonTrigger^>
    echo   ^</Triggers^>
    echo   ^<Principals^>
    echo     ^<Principal id="Author"^>
    echo       ^<GroupId^>S-1-5-32-545^</GroupId^>
    echo       ^<RunLevel^>LeastPrivilege^</RunLevel^>
    echo     ^</Principal^>
    echo   ^</Principals^>
    echo   ^<Settings^>
    echo     ^<MultipleInstancesPolicy^>IgnoreNew^</MultipleInstancesPolicy^>
    echo     ^<DisallowStartIfOnBatteries^>false^</DisallowStartIfOnBatteries^>
    echo     ^<StopIfGoingOnBatteries^>false^</StopIfGoingOnBatteries^>
    echo     ^<AllowHardTerminate^>true^</AllowHardTerminate^>
    echo     ^<StartWhenAvailable^>true^</StartWhenAvailable^>
    echo     ^<RunOnlyIfNetworkAvailable^>false^</RunOnlyIfNetworkAvailable^>
    echo     ^<AllowStartOnDemand^>true^</AllowStartOnDemand^>
    echo     ^<Enabled^>true^</Enabled^>
    echo     ^<ExecutionTimeLimit^>PT0S^</ExecutionTimeLimit^>
    echo     ^<Priority^>4^</Priority^>
    echo     ^<RestartOnFailure^>
    echo       ^<Interval^>PT1M^</Interval^>
    echo       ^<Count^>99^</Count^>
    echo     ^</RestartOnFailure^>
    echo   ^</Settings^>
    echo   ^<Actions Context="Author"^>
    echo     ^<Exec^>
    echo       ^<Command^>!AGENT_BIN!^</Command^>
    echo     ^</Exec^>
    echo   ^</Actions^>
    echo ^</Task^>
) > "!TASK_XML!"

schtasks /Create /XML "!TASK_XML!" /TN "ActivityMonitorUserAgent" /F >nul 2>&1
set SCHTASK_ERR=!errorLevel!
if exist "!TASK_XML!" del /F /Q "!TASK_XML!" >nul 2>&1

if !SCHTASK_ERR! equ 0 (
    echo    [+] Tarea de usuario creada
) else (
    schtasks /Create /SC ONLOGON /TN "ActivityMonitorUserAgent" /TR "\"%AGENT_BIN%\"" /F /IT >nul 2>&1
    if !errorLevel! equ 0 (
        echo    [+] Tarea de usuario creada (fallback)
    ) else (
        echo  [!] No se pudo crear la tarea programada
    )
)

REM ---- 7. Start ----
echo  [7/7] Iniciando agente...
sc start ActivityMonitor >nul 2>&1
schtasks /Run /TN "ActivityMonitorUserAgent" >nul 2>&1
echo    [+] Agente iniciado

REM ---- Completion ----
cls
echo ============================================================
echo     INSTALACION COMPLETADA EXITOSAMENTE
echo ============================================================
echo.
echo   Binario:  %AGENT_BIN%
echo   Config:   %ENV_FILE%
echo   Servidor: !AGENT_SERVER_URL!
echo   Logs:     %LOG_DIR%
echo.
echo   El agente ya esta reportando en segundo plano.
echo   Puedes cerrar esta ventana.
echo.
echo   Para verificar el estado:
echo     sc query ActivityMonitor
echo     schtasks /Query /TN ActivityMonitorUserAgent
echo.
echo   Para desinstalar, ejecuta: install.bat /uninstall
echo.
pause
exit /b 0

REM ============================================================
REM HELP
REM ============================================================
:HELP
cls
echo ============================================================
echo     ActivityMonitor Agent v3 - Ayuda
echo ============================================================
echo.
echo   install.bat              Instala el agente (modo interactivo)
echo   install.bat /uninstall   Desinstala completamente el agente
echo   install-silent.bat       Instalacion silenciosa (sin preguntas)
echo.
echo   REQUISITOS:
echo     - activity-monitor-agent.exe (pre-compilado)
echo     - agent-config.json (configuracion, opcional)
echo.
echo   Archivos en el USB:
echo     install.bat              Este instalador
echo     install-silent.bat       Instalador silencioso
echo     activity-monitor-agent.exe  Binario del agente
echo     agent-config.json        Configuracion del agente
echo     README.txt               Instrucciones
echo.
pause
exit /b 0

REM ============================================================
REM UNINSTALL
REM ============================================================
:UNINSTALL
cls
echo ============================================================
echo     Desinstalando ActivityMonitor Agent...
echo ============================================================
echo.

echo  [*] Deteniendo servicio...
net stop ActivityMonitor >nul 2>&1
sc delete ActivityMonitor >nul 2>&1

echo  [*] Deteniendo procesos...
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1

echo  [*] Eliminando tarea programada...
schtasks /Delete /TN ActivityMonitorUserAgent /F >nul 2>&1

echo  [*] Limpiando archivos...
rmdir /s /q "%PROGRAMDATA%\ActivityMonitor" >nul 2>&1

echo.
echo  [+] Desinstalacion completada.
pause
exit /b 0
