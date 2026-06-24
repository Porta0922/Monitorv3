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
    browser_download_url: String,
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

    let resp = match client.get(GITHUB_API).send() {
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
                    download_url: a.browser_download_url.clone(),
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

    let resp = client.get(url).send()?;
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
         echo [*] Stopping service and user agent...\r\n\
         sc stop \"{service}\" >nul 2>&1\r\n\
         taskkill /F /IM activity-monitor-agent.exe >nul 2>&1\r\n\
         :wait\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         tasklist /FI \"IMAGENAME eq activity-monitor-agent.exe\" 2>nul | find /I \"activity-monitor-agent.exe\" >nul\r\n\
         if not errorlevel 1 goto wait\r\n\
         echo [*] Copying new binary...\r\n\
         copy /Y \"{new}\" \"{exe}\" >nul\r\n\
         if errorlevel 1 (\r\n\
             echo [-] ERROR: Failed to copy binary\r\n\
             pause\r\n\
             exit /b 1\r\n\
         )\r\n\
         echo [+] Binary updated to {version}\r\n\
         echo [*] Starting service...\r\n\
         sc start \"{service}\" >nul 2>&1\r\n\
         schtasks /Run /TN \"{task}\" >nul 2>&1\r\n\
         echo [+] Service started\r\n\
         echo [+] Update complete\r\n\
         del /F /Q \"{new}\" >nul 2>&1\r\n\
         del /F /Q \"%~f0\" >nul 2>&1\r\n",
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
