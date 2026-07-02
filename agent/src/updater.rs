use reqwest::StatusCode;
use serde::Deserialize;

const GITHUB_API: &str = "https://api.github.com/repos/Porta0922/Monitorv3/releases/latest";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    url: String,
}

#[derive(Debug, PartialEq)]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable { version: String, download_url: String },
    Error(String),
}

pub fn check_for_update() -> UpdateStatus {
    let client = match reqwest::blocking::Client::builder()
        .user_agent("ActivityMonitor-Agent")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return UpdateStatus::Error(format!("Error creating HTTP client: {}", e)),
    };

    let token = std::env::var("GITHUB_TOKEN").ok();

    let mut req = client.get(GITHUB_API);
    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = match req.send() {
        Ok(r) => r,
        Err(e) => return UpdateStatus::Error(format!("Error contacting GitHub API: {}", e)),
    };

    if resp.status() == StatusCode::NOT_FOUND {
        return UpdateStatus::UpToDate;
    }
    if !resp.status().is_success() {
        return UpdateStatus::Error(format!(
            "GitHub API returned HTTP {}",
            resp.status()
        ));
    }

    let release: GitHubRelease = match resp.json() {
        Ok(r) => r,
        Err(e) => return UpdateStatus::Error(format!("Error parsing GitHub response: {}", e)),
    };

    let latest_tag = release.tag_name.trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");

    match compare_versions(latest_tag, current) {
        std::cmp::Ordering::Greater => {
            let asset_name = "activity-monitor-agent.exe";

            let asset = release.assets.iter().find(|a| a.name == asset_name);
            match asset {
                Some(a) => UpdateStatus::UpdateAvailable {
                    version: release.tag_name.clone(),
                    download_url: a.url.clone(),
                },
                None => UpdateStatus::Error(format!(
                    "Asset '{}' not found in release {}",
                    asset_name, release.tag_name
                )),
            }
        }
        std::cmp::Ordering::Equal | std::cmp::Ordering::Less => UpdateStatus::UpToDate,
    }
}

pub fn download_and_install(url: &str, dest: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("ActivityMonitor-Agent")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let token = std::env::var("GITHUB_TOKEN").ok();

    // Use the API asset URL with Accept: application/octet-stream to get the binary
    // directly (avoids losing auth headers on CDN redirects)
    let mut req = client.get(url)
        .header("Accept", "application/octet-stream");
    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send()?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with HTTP {}", resp.status()).into());
    }

    let bytes = resp.bytes()?;
    std::fs::write(dest, &bytes)?;
    Ok(())
}

pub fn create_update_script(
    new_binary: &std::path::Path,
    service_name: &str,
    version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let script_path = std::env::temp_dir().join("am_update.bat");
    let current_exe = std::env::current_exe()?;

    let script = format!(
        "@echo off\r\n\
         setlocal enabledelayedexpansion\r\n\
         REM ActivityMonitor self-updater\r\n\
         \r\n\
         echo [*] Stopping service and user agent...\r\n\
         \r\n\
         REM Disable service recovery to prevent auto-restart during update\r\n\
         sc failure \"{service}\" reset=86400 actions= \"\" >nul 2>&1\r\n\
         \r\n\
         REM Check if service exists; create it if missing\r\n\
         sc query \"{service}\" >nul 2>&1\r\n\
         if errorlevel 1 (\r\n\
             echo [*] Service not found, creating...\r\n\
             sc create \"{service}\" binPath= \"{exe}\" start= delayed-auto displayName= \"ActivityMonitor Enterprise Agent\" >nul 2>&1\r\n\
             sc failure \"{service}\" reset=86400 actions= restart/5000/restart/10000/restart/30000 >nul 2>&1\r\n\
         )\r\n\
         \r\n\
         REM Try to stop service gracefully, then force-kill regardless\r\n\
         sc stop \"{service}\" >nul 2>&1\r\n\
         \r\n\
         REM Wait for service with 30-second timeout\r\n\
         set wait_secs=0\r\n\
         :wait_stop\r\n\
         timeout /t 3 /nobreak >nul\r\n\
         set /a wait_secs+=3\r\n\
         sc query \"{service}\" 2>nul | find \"STOPPED\" >nul 2>&1\r\n\
         if not errorlevel 1 goto stopped\r\n\
         if !wait_secs! lss 30 goto wait_stop\r\n\
         echo [*] Service did not stop gracefully, force-killing...\r\n\
         \r\n\
         :stopped\r\n\
         echo [*] Killing remaining processes...\r\n\
         taskkill /F /IM activity-monitor-agent.exe >nul 2>&1\r\n\
         timeout /t 3 /nobreak >nul\r\n\
         taskkill /F /IM activity-monitor-agent.exe >nul 2>&1\r\n\
         \r\n\
         REM Rename the in-use file first (Windows allows this), then copy new one\r\n\
         echo [*] Copying new binary...\r\n\
         set retries=0\r\n\
         :copy_retry\r\n\
         if exist \"{exe}\" (\r\n\
             rename \"{exe}\" \"activity-monitor-agent.exe.old\" >nul 2>&1\r\n\
         )\r\n\
         copy /Y \"{new}\" \"{exe}\" >nul 2>&1\r\n\
         if errorlevel 1 (\r\n\
             set /a retries+=1\r\n\
             if !retries! lss 5 (\r\n\
                 timeout /t 2 /nobreak >nul\r\n\
                 goto copy_retry\r\n\
             )\r\n\
             echo [-] ERROR: Failed to copy binary after !retries! attempts\r\n\
             pause\r\n\
             exit /b 1\r\n\
         )\r\n\
         if exist \"{exe}.old\" del /F /Q \"{exe}.old\" >nul 2>&1\r\n\
         echo [+] Binary updated to {version}\r\n\
         \r\n\
         REM Restore service recovery\r\n\
         sc failure \"{service}\" reset=86400 actions= restart/5000/restart/10000/restart/30000 >nul 2>&1\r\n\
         \r\n\
         echo [*] Starting service...\r\n\
         sc start \"{service}\" >nul 2>&1\r\n\
         schtasks /Run /TN \"{task}\" >nul 2>&1\r\n\
         echo [+] Service started\r\n\
         echo [+] Update complete\r\n\
          del /F /Q \"{new}\" >nul 2>&1\r\n\
          cmd /c del /f /q \"%~f0\" >nul 2>&1\r\n\
          exit\r\n",
        new = new_binary.display(),
        exe = current_exe.display(),
        service = service_name,
        task = "ActivityMonitorUserAgent",
        version = version,
    );

    std::fs::write(&script_path, script)?;
    Ok(script_path)
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();
    let b_parts: Vec<u32> = b
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();

    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_val = a_parts.get(i).copied().unwrap_or(0);
        let b_val = b_parts.get(i).copied().unwrap_or(0);
        if a_val > b_val {
            return std::cmp::Ordering::Greater;
        }
        if a_val < b_val {
            return std::cmp::Ordering::Less;
        }
    }
    std::cmp::Ordering::Equal
}
