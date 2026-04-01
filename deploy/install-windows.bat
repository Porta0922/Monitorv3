@echo off
REM ActivityMonitor Enterprise v3 - Windows Installer
REM Registers agent as Windows Service using NSSM (Non-Sucking Service Manager)

SETLOCAL ENABLEDELAYEDEXPANSION

REM Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo This script requires Administrator privileges.
    pause
    exit /b 1
)

REM Configuration
set AGENT_NAME=ActivityMonitorAgent
set AGENT_VERSION=0.1.0
set SERVICE_NAME=ActivityMonitor
set AGENT_PATH=%~dp0\..\target\release\activity-monitor-agent.exe
set NSSM_PATH=%~dp0\nssm.exe
set CONFIG_DIR=%PROGRAMDATA%\ActivityMonitor
set LOG_DIR=%PROGRAMDATA%\ActivityMonitor\logs
set ENV_FILE=%CONFIG_DIR%\.env

echo ========================================
echo ActivityMonitor Enterprise v3 Installer
echo ========================================
echo.

REM Ask for device nickname
set /p DEVICE_NICKNAME="Enter device nickname (or press Enter for auto): "
if "!DEVICE_NICKNAME!"=="" (
    for /f "tokens=*" %%A in ('hostname') do set DEVICE_NICKNAME=%%A
    echo Using hostname: !DEVICE_NICKNAME!
)

REM Create config directory
if not exist "%CONFIG_DIR%" mkdir "%CONFIG_DIR%"
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
echo [+] Created config directory: %CONFIG_DIR%

REM Create .env file with device nickname
if not exist "%ENV_FILE%" (
    echo Creating configuration file...
    (
        echo # ActivityMonitor Agent Configuration
        echo DEVICE_NICKNAME=!DEVICE_NICKNAME!
        echo SERVER_URL=http://localhost:3000
        echo RABBITMQ_URL=amqp://guest:guest@localhost:5672/%%2F
    ) > "%ENV_FILE%"
    echo [+] Created configuration: %ENV_FILE%
) else (
    echo [!] Configuration file already exists
)

REM Check if agent binary exists
if not exist "%AGENT_PATH%" (
    echo [-] Agent binary not found at %AGENT_PATH%
    echo [!] Please build the agent first: cargo build --release
    pause
    exit /b 1
)
echo [+] Found agent binary: %AGENT_PATH%

REM Download NSSM if not present
if not exist "%NSSM_PATH%" (
    echo [*] Downloading NSSM...
    powershell -Command "(New-Object System.Net.WebClient).DownloadFile('https://nssm.cc/download/nssm-2.24-101-g897c7ad.zip', '%~dp0nssm.zip')"
    powershell -Command "Expand-Archive '%~dp0nssm.zip' -DestinationPath '%~dp0'"
    copy "%~dp0nssm-2.24-101-g897c7ad\win64\nssm.exe" "%NSSM_PATH%"
    echo [+] NSSM installed
)

REM Stop and remove existing service
echo [*] Checking for existing service...
sc query %SERVICE_NAME% >nul 2>&1
if %errorLevel% equ 0 (
    echo [*] Stopping existing service...
    net stop %SERVICE_NAME% >nul 2>&1
    echo [*] Removing existing service...
    %NSSM_PATH% remove %SERVICE_NAME% confirm
)

REM Install new service
echo [*] Installing service...
%NSSM_PATH% install %SERVICE_NAME% "%AGENT_PATH%"
%NSSM_PATH% set %SERVICE_NAME% AppDirectory "%CONFIG_DIR%"
%NSSM_PATH% set %SERVICE_NAME% AppStdout "%LOG_DIR%\output.log"
%NSSM_PATH% set %SERVICE_NAME% AppStderr "%LOG_DIR%\error.log"
%NSSM_PATH% set %SERVICE_NAME% AppRotateFiles 1
%NSSM_PATH% set %SERVICE_NAME% AppRotateOnline 1
%NSSM_PATH% set %SERVICE_NAME% AppRotateSeconds 86400
%NSSM_PATH% set %SERVICE_NAME% AppRotateBytes 10485760

REM Set environment variables for service
%NSSM_PATH% set %SERVICE_NAME% AppEnvironmentExtra "DEVICE_NICKNAME=!DEVICE_NICKNAME!"

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

echo.
echo ========================================
echo Installation Complete
echo ========================================
echo Device Nickname: !DEVICE_NICKNAME!
echo Service Name: %SERVICE_NAME%
echo Binary Path: %AGENT_PATH%
echo Config Dir: %CONFIG_DIR%
echo Log Dir: %LOG_DIR%
echo.
echo To manage the service:
echo   Start:   net start ActivityMonitor
echo   Stop:    net stop ActivityMonitor
echo   Change nickname: Edit %ENV_FILE%
echo   Uninstall: nssm remove ActivityMonitor confirm
echo.
pause
