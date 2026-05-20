@echo off
REM ActivityMonitor Enterprise v3 - Windows SILENT Installer (USB / AnyDesk)
REM Installation completed in 1 second without questions.

SETLOCAL ENABLEDELAYEDEXPANSION
set SERVICE_NAME=ActivityMonitor

REM Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [ERR] Este script requiere privilegios de Administrador.
    echo [*] Solicitando elevacion de privilegios...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process cmd -ArgumentList '/c \"%~f0\"' -Verb RunAs"
    exit /b 1
)

REM Configuration
set AGENT_VERSION=3.3.3
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor
set INSTALL_DIR=%PROGRAMDATA%\ActivityMonitor
set BIN_DIR=%PROGRAMDATA%\ActivityMonitor\Bin
set DATA_DIR=%PROGRAMDATA%\ActivityMonitor\Data
set AGENT_BIN=%BIN_DIR%\activity-monitor-agent.exe
set LOG_DIR=%PROGRAMDATA%\ActivityMonitor\logs
set ENV_FILE=%CONFIG_DIR%\.env

REM Master Credentials (Modificar aqui para pre-configurar tus despliegues masivos)
set AGENT_AUTH_TOKEN=change-me-in-production
set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
set AGENT_SERVER_URL=http://10.30.0.123:3000
set RABBITMQ_URL=amqp://eclub:eCLUB123@10.30.0.123:5672/%%2f

REM Locate precompiled binary
if exist "%~dp0activity-monitor-agent.exe" (
    set AGENT_SRC=%~dp0activity-monitor-agent.exe
) else if exist "%~dp0..\activity-monitor-agent.exe" (
    set AGENT_SRC=%~dp0..\activity-monitor-agent.exe
) else if exist "%~dp0..\..\target\release\activity-monitor-agent.exe" (
    set AGENT_SRC=%~dp0..\..\target\release\activity-monitor-agent.exe
) else (
    echo [-] ERROR: No se encontro el binario activity-monitor-agent.exe pre-compilado en el USB.
    echo [*] Por favor copie el ejecutable compilado al lado de este script.
    pause
    exit /b 1
)

echo [*] Iniciando instalacion desatendida...

REM 1. Crear directorios
if not exist "%INSTALL_DIR%" mkdir "%INSTALL_DIR%"
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%DATA_DIR%" mkdir "%DATA_DIR%"
icacls "%INSTALL_DIR%" /grant:r *S-1-5-32-545:(OI)(CI)M /T >nul 2>&1

REM 2. Escribir .env silencioso
(
    echo # ActivityMonitor Agent Configuration
    echo AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!
    echo AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!
    echo AGENT_SERVER_URL=!AGENT_SERVER_URL!
    echo RABBITMQ_URL=!RABBITMQ_URL!
) > "%ENV_FILE%"

REM 3. Detener agentes viejos
taskkill /F /IM activity-monitor-agent.exe >nul 2>&1
sc stop ActivityMonitor >nul 2>&1
sc delete ActivityMonitor >nul 2>&1

REM 4. Copiar binario
copy /Y "%AGENT_SRC%" "%AGENT_BIN%" >nul 2>&1

REM 5. Registrar Servicio de Windows (Sesion 0)
sc create ActivityMonitor binPath= "\"%AGENT_BIN%\"" start= delayed-auto displayName= "ActivityMonitor Enterprise Agent" >nul 2>&1

REM 6. Registrar Tarea Programada de Usuario via XML (Mitigacion de Antivirus)
set "TASK_XML=%TEMP%\ActivityMonitorTask.xml"
(
    echo ^<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task"^>
    echo   ^<RegistrationInfo^>
    echo     ^<Date^>2026-05-20T12:00:00^</Date^>
    echo     ^<Author^>ActivityMonitor^</Author^>
    echo     ^<Description^>ActivityMonitor User Agent persistence task^</Description^>
    echo   ^</RegistrationInfo^>
    echo   ^<Triggers^>
    echo     ^<LogonTrigger^>
    echo       ^<Enabled^>true^</Enabled^>
    echo     ^</LogonTrigger^>
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
    echo     ^<IdleSettings^>
    echo       ^<StopOnIdleEnd^>true^</StopOnIdleEnd^>
    echo       ^<RestartOnIdle^>false^</RestartOnIdle^>
    echo     ^</IdleSettings^>
    echo     ^<AllowStartOnDemand^>true^</AllowStartOnDemand^>
    echo     ^<Enabled^>true^</Enabled^>
    echo     ^<Hidden^>false^</Hidden^>
    echo     ^<RunOnlyIfIdle^>false^</RunOnlyIfIdle^>
    echo     ^<WakeToRun^>false^</WakeToRun^>
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
if exist "!TASK_XML!" del /F /Q "!TASK_XML!" >nul 2>&1

REM 7. Levantar Procesos
sc start ActivityMonitor >nul 2>&1
schtasks /Run /TN "ActivityMonitorUserAgent" >nul 2>&1

echo [+] INSTALACION COMPLETADA EXITOSAMENTE.
echo [*] Agente reportando activamente en segundo plano.
timeout /t 3 >nul
exit /b 0
