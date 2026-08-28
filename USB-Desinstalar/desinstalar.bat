@echo off
:: ============================================================
::  ActivityMonitor - Desinstalar desde USB
::  No requiere GitHub ni token. Corre offline.
:: ============================================================
if not exist "%~dp0activity-monitor-agent.exe" (
    echo ERROR: no se encuentra activity-monitor-agent.exe junto a este .bat
    pause
    exit /b 1
)
copy /y "%~dp0activity-monitor-agent.exe" "%TEMP%\am_remove.exe" >nul
echo Ejecutando removedor...
"%TEMP%\am_remove.exe"
if exist "%TEMP%\am_remove.exe" del /q "%TEMP%\am_remove.exe" >nul 2>&1
echo.
echo Fin. Ejecute:  sc query ActivityMonitor  (debe dar "no existe")
pause