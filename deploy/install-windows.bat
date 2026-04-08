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
set OSQUERY_VERSION=5.22.1
set OSQUERY_MSI_URL=https://github.com/osquery/osquery/releases/download/%OSQUERY_VERSION%/osquery-%OSQUERY_VERSION%.msi
set OSQUERY_MSI_PATH=%TEMP%\osquery-%OSQUERY_VERSION%.msi
set AGENT_AUTH_TOKEN=dev-agent-token
set AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!

echo ========================================
echo ActivityMonitor Enterprise v3 Installer
echo ========================================
echo.

REM Ask for optional auth token (used by current agent)
set /p INPUT_AUTH_TOKEN="Enter agent auth token (or press Enter for default dev-agent-token): "
if not "!INPUT_AUTH_TOKEN!"=="" (
    set AGENT_AUTH_TOKEN=!INPUT_AUTH_TOKEN!
)

REM Create config directory
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

REM Install osquery if not present
if not exist "C:\Program Files\osquery\osqueryi.exe" (
    echo [*] osquery not found. Installing...

    where choco >nul 2>&1
    if !errorLevel! equ 0 (
        echo [*] Chocolatey detected. Installing osquery from Chocolatey repository...
        choco install osquery -y --no-progress
    ) else (
        echo [!] Chocolatey not found. Falling back to direct MSI download (%OSQUERY_VERSION%)...
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
%NSSM_PATH% set %SERVICE_NAME% AppEnvironmentExtra "AGENT_AUTH_TOKEN=!AGENT_AUTH_TOKEN!" "AGENT_OFFLINE_CACHE_KEY=!AGENT_OFFLINE_CACHE_KEY!"

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
echo Agent auth token configured
echo Service Name: %SERVICE_NAME%
echo Binary Path: %AGENT_PATH%
echo Config Dir: %CONFIG_DIR%
echo Log Dir: %LOG_DIR%
echo.
echo To manage the service:
echo   Start:   net start ActivityMonitor
echo   Stop:    net stop ActivityMonitor
echo   Update token/key: Edit %ENV_FILE% and reinstall service
echo   Uninstall: nssm remove ActivityMonitor confirm
echo.
pause
