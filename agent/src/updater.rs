use reqwest::StatusCode;
use serde::Deserialize;
use sha2::Digest;

const GITHUB_API: &str = "https://api.github.com/repos/Porta0922/Monitorv3/releases/latest";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    body: String,
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
    UpdateAvailable {
        version: String,
        download_url: String,
        sha256: String,
    },
    Error(String),
}

pub async fn check_for_update() -> UpdateStatus {
    let client = match reqwest::Client::builder()
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

    let resp = match req.send().await {
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

    let release: GitHubRelease = match resp.json().await {
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
                Some(a) => {
                    let sha256 = parse_sha256_from_body(&release.body);
                    match sha256 {
                        Some(s) => UpdateStatus::UpdateAvailable {
                            version: release.tag_name.clone(),
                            download_url: a.url.clone(),
                            sha256: s,
                        },
                        None => UpdateStatus::Error(format!(
                            "SHA256 not found in release body for {}",
                            release.tag_name
                        )),
                    }
                }
                None => UpdateStatus::Error(format!(
                    "Asset '{}' not found in release {}",
                    asset_name, release.tag_name
                )),
            }
        }
        std::cmp::Ordering::Equal | std::cmp::Ordering::Less => UpdateStatus::UpToDate,
    }
}

fn parse_sha256_from_body(body: &str) -> Option<String> {
    let idx = body.find("SHA256:")?;
    let rest = &body[idx + "SHA256:".len()..];
    let hash: String = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit() || *c == ' ')
        .collect();
    let hash = hash.trim();
    if hash.len() == 64 {
        Some(hash.to_lowercase())
    } else {
        None
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    hex::encode(out)
}

pub async fn download_and_install(
    url: &str,
    dest: &std::path::Path,
    expected_sha256: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
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

    let resp = req.send().await?;
    if !resp.status().is_success() {
        return Err(format!("Download failed with HTTP {}", resp.status()).into());
    }

    let bytes = resp.bytes().await?;

    let actual = sha256_hex(&bytes);
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "SHA256 mismatch: expected {}, got {}. Update aborted.",
            expected_sha256, actual
        )
        .into());
    }

    std::fs::write(dest, &bytes)?;
    Ok(())
}

pub fn create_update_script(
    new_binary: &std::path::Path,
    service_name: &str,
    version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    create_regular_update_script(new_binary, service_name, version)
}

#[derive(Debug)]
pub enum ApplyUpdateResult {
    UpToDate,
    Updated { version: String },
    Failed(String),
}

/// Runs the full update pipeline: check -> download (+SHA256 verify) -> apply script.
pub async fn apply_update() -> ApplyUpdateResult {
    match check_for_update().await {
        UpdateStatus::UpToDate => ApplyUpdateResult::UpToDate,
        UpdateStatus::UpdateAvailable { version, download_url, sha256 } => {
            tracing::info!("[AutoUpdate] Update v{} available, downloading...", version);
            let temp_dir = std::env::temp_dir();
            let dest_path = temp_dir.join("am_update.exe");

            if let Err(e) = download_and_install(&download_url, &dest_path, &sha256).await {
                return ApplyUpdateResult::Failed(format!("download/verify failed: {}", e));
            }

            let service_name = if cfg!(windows) { "ActivityMonitor" } else { "activity-monitor" };
            match create_update_script(&dest_path, service_name, &version) {
                Ok(script_path) => {
                    tracing::info!("[AutoUpdate] Spawning cmd.exe for update script: {}", script_path.display());
                    if spawn_update_script(&script_path).is_some() {
                        ApplyUpdateResult::Updated { version }
                    } else {
                        ApplyUpdateResult::Failed("failed to spawn update script".to_string())
                    }
                }
                Err(e) => ApplyUpdateResult::Failed(format!("create_update_script failed: {}", e)),
            }
        }
        UpdateStatus::Error(e) => ApplyUpdateResult::Failed(e),
    }
}

/// Spawns cmd.exe running the update batch script. No window, breaks away from the
/// current job object so it can survive service shutdown.
#[cfg(windows)]
pub fn spawn_update_script(script_path: &std::path::Path) -> Option<std::process::Child> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;

    let mut cmd = std::process::Command::new("cmd.exe");
    cmd.args(&["/c", &script_path.to_string_lossy()]);
    cmd.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
    cmd.spawn().ok()
}

#[cfg(not(windows))]
pub fn spawn_update_script(script_path: &std::path::Path) -> Option<std::process::Child> {
    std::process::Command::new("sh")
        .arg(script_path)
        .spawn()
        .ok()
}

fn create_regular_update_script(
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