use std::path::Path;

const SERVICE_NAME: &str = "ActivityMonitor";
const TASK_NAME: &str = "ActivityMonitorUserAgent";
const DATA_DIR: &str = r"C:\ProgramData\ActivityMonitor";
const REG_KEY: &str = r"HKLM\SOFTWARE\ActivityMonitor";

/// Writes a self-elevating batch script to temp and spawns it detached.
/// The script removes the service, scheduled tasks, processes, data and
/// registry, then deletes the agent binary and the script itself.
pub fn spawn_uninstall() -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let script_path = std::env::temp_dir().join("am_uninstall.bat");

    let exe = current_exe.display().to_string();
    let script = build_uninstall_script(&exe);

    std::fs::write(&script_path, script)?;
    spawn_bat(&script_path)?;
    Ok(())
}

fn build_uninstall_script(exe: &str) -> String {
    format!(
        "@echo off\r\n\
         setlocal enabledelayedexpansion\r\n\
         REM ActivityMonitor Enterprise - Self uninstall (v340)\r\n\
         \r\n\
         REM Re-launch elevated if we are not admin (covers the tray/user path)\r\n\
         net session >nul 2>&1\r\n\
         if errorlevel 1 (\r\n\
             echo [*] Requesting administrator privileges...\r\n\
             powershell -NoProfile -ExecutionPolicy Bypass -Command \"Start-Process -FilePath '%~f0' -Verb RunAs\" >nul 2>&1\r\n\
             exit /b\r\n\
         )\r\n\
         \r\n\
         echo [*] Disabling service recovery (prevents auto-restart)...\r\n\
         sc failure \"{service}\" reset=86400 actions= \"\" >nul 2>&1\r\n\
         \r\n\
         echo [*] Deleting scheduled tasks...\r\n\
         schtasks /Delete /TN \"{task}\" /F >nul 2>&1\r\n\
         schtasks /Query /FO CSV /NH 2>nul | findstr /I \"ActivityMonitor\" >nul 2>&1\r\n\
         if not errorlevel 1 (\r\n\
             for /f \"tokens=1 delims=,\" %%T in ('schtasks /Query /FO CSV /NH ^| findstr /I \"ActivityMonitor\"') do (\r\n\
                 schtasks /Delete /TN %%T /F >nul 2>&1\r\n\
             )\r\n\
         )\r\n\
         \r\n\
         echo [*] Stopping and deleting service...\r\n\
         sc stop \"{service}\" >nul 2>&1\r\n\
         sc delete \"{service}\" >nul 2>&1\r\n\
         \r\n\
         echo [*] Killing remaining agent processes...\r\n\
         taskkill /F /IM activity-monitor-agent.exe >nul 2>&1\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         taskkill /F /IM activity-monitor-agent.exe >nul 2>&1\r\n\
         \r\n\
         echo [*] Removing data directory...\r\n\
         rmdir /s /q \"{data}\" >nul 2>&1\r\n\
         \r\n\
         echo [*] Cleaning registry...\r\n\
         reg delete \"{reg}\" /f >nul 2>&1\r\n\
         \r\n\
         echo [*] Removing agent binary...\r\n\
         del /F /Q \"{exe}\" >nul 2>&1\r\n\
         if exist \"{exe}\" rmdir /s /q \"%~dp0\" >nul 2>&1\r\n\
         \r\n\
         echo [+] Uninstall complete\r\n\
         cmd /c del /f /q \"%~f0\" >nul 2>&1\r\n\
         exit\r\n",
        service = SERVICE_NAME,
        task = TASK_NAME,
        data = DATA_DIR,
        reg = REG_KEY,
        exe = exe,
    )
}

fn spawn_bat(script_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = std::process::Command::new("cmd.exe");
    cmd.args(&["/c", &script_path.to_string_lossy()]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
    }
    cmd.spawn()?;
    Ok(())
}