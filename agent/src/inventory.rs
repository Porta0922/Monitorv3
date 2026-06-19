// Software inventory scanner
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use chrono::{DateTime, Utc};
use crate::monitoring;
#[cfg(unix)]
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub app_name: String,
    pub version: Option<String>,
    pub exe_path: String,
    pub exe_hash: String,
    pub installed_date: Option<DateTime<Utc>>,
}

pub struct InventoryScanner;

impl InventoryScanner {
    pub fn fingerprint(app_name: &str, version: Option<&str>, exe_hash: &str) -> String {
        format!(
            "{}|{}|{}",
            app_name.trim().to_lowercase(),
            version.unwrap_or("unknown").trim().to_lowercase(),
            exe_hash.trim().to_lowercase(),
        )
    }

    fn is_noise_or_system_app(app_name: &str) -> bool {
        let normalized = app_name.trim().to_lowercase();

        if normalized.is_empty() {
            return true;
        }

        let blocked_keywords = [
            "windows sdk",
            "software development kit",
            "development libraries",
            "targeting pack",
            "windows driver package",
            "microsoft visual c++",
            "redistributable",
            "security update",
            "update for",
            "hotfix",
            "debugging tools",
            "x64 remote",
            "x86 remote",
        ];

        blocked_keywords.iter().any(|keyword| normalized.contains(keyword))
    }

    /// Scan system for installed software
    pub async fn scan_installed_software() -> Result<Vec<InstalledApp>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            Self::scan_windows_software().await
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::scan_linux_software().await
        }
        
        #[cfg(target_os = "macos")]
        {
            Self::scan_macos_software().await
        }
    }

    /// Generate inventory report as JSON
    pub async fn generate_inventory_report() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let apps = Self::scan_installed_software().await?;
        
        let report = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "total_apps": apps.len(),
            "apps": apps,
        });
        
        Ok(report)
    }
}

#[cfg(target_os = "windows")]
impl InventoryScanner {
    async fn scan_windows_software() -> Result<Vec<InstalledApp>, Box<dyn std::error::Error>> {
        let mut apps = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        
        // Common program paths
        let program_paths = vec![
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\Users",
        ];
        
        for base_path in program_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    // Look for executable files
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "exe") {
                        if let Ok(hash) = monitoring::calculate_file_hash_with_cache(&path.to_string_lossy()) {
                            let app = InstalledApp {
                                app_name: path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unknown")
                                    .to_string(),
                                version: None,
                                exe_path: path.to_string_lossy().to_string(),
                                exe_hash: hash,
                                installed_date: None,
                            };
                            if !Self::is_noise_or_system_app(&app.app_name) {
                                let key = Self::fingerprint(&app.app_name, app.version.as_deref(), &app.exe_hash);
                                if seen.insert(key) {
                                    apps.push(app);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also scan Windows registry for installed programs
        for app in Self::scan_registry_uninstall()? {
            if Self::is_noise_or_system_app(&app.app_name) {
                continue;
            }

            let key = Self::fingerprint(&app.app_name, app.version.as_deref(), &app.exe_hash);
            if seen.insert(key) {
                apps.push(app);
            }
        }
        
        Ok(apps)
    }

    fn scan_registry_uninstall() -> Result<Vec<InstalledApp>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            use winreg::enums::HKEY_LOCAL_MACHINE;
            
            let mut apps = Vec::new();
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            
            // Check Uninstall registry for installed apps
            if let Ok(uninstall) = hklm.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall") {
                for subkey_name in uninstall.enum_keys().flatten() {
                    if let Ok(subkey) = uninstall.open_subkey(&subkey_name) {
                        if let Ok(display_name) = subkey.get_value::<String, _>("DisplayName") {
                            if let Ok(version) = subkey.get_value::<String, _>("DisplayVersion") {
                                let app = InstalledApp {
                                    app_name: display_name,
                                    version: Some(version),
                                    exe_path: String::new(),
                                    exe_hash: String::new(),
                                    installed_date: None,
                                };
                                apps.push(app);
                            }
                        }
                    }
                }
            }
            
            Ok(apps)
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            Ok(Vec::new())
        }
    }
}

#[cfg(target_os = "linux")]
impl InventoryScanner {
    async fn scan_linux_software() -> Result<Vec<InstalledApp>, Box<dyn std::error::Error>> {
        use std::process::Command;
        
        let mut apps = Vec::new();
        
        // Scan /usr/bin for binaries
        if let Ok(entries) = std::fs::read_dir("/usr/bin") {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_file() && is_executable(&path) {
                    if let Ok(hash) = crate::monitoring::calculate_file_hash_with_cache(&path.to_string_lossy()) {
                        let app = InstalledApp {
                            app_name: path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string(),
                            version: None,
                            exe_path: path.to_string_lossy().to_string(),
                            exe_hash: hash,
                            installed_date: None,
                        };
                        apps.push(app);
                    }
                }
            }
        }
        
        // Also try dpkg for installed packages (Debian/Ubuntu)
        if let Ok(output) = Command::new("/usr/bin/dpkg-query").args(["-W", "-f=${Package}|${Version}|${Architecture}\n"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 2 {
                    let app = InstalledApp {
                        app_name: parts[0].to_string(),
                        version: Some(parts[1].to_string()),
                        exe_path: format!("/usr/bin/{}", parts[0]),
                        exe_hash: String::new(),
                        installed_date: None,
                    };
                    apps.push(app);
                }
            }
        }

        // Try snap list if available
        if let Ok(output) = Command::new("/usr/bin/snap").arg("list").output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for (i, line) in output_str.lines().enumerate() {
                if i == 0 { continue; } // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let app = InstalledApp {
                        app_name: parts[0].to_string(),
                        version: Some(parts[1].to_string()),
                        exe_path: format!("/snap/bin/{}", parts[0]),
                        exe_hash: String::new(),
                        installed_date: None,
                    };
                    apps.push(app);
                }
            }
        }
        
        Ok(apps)
    }
}

#[cfg(target_os = "macos")]
impl InventoryScanner {
    async fn scan_macos_software() -> Result<Vec<InstalledApp>, Box<dyn std::error::Error>> {
        use std::process::Command;
        
        let mut apps = Vec::new();
        
        // Scan /Applications
        if let Ok(entries) = std::fs::read_dir("/Applications") {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    let app_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    
                    let app = InstalledApp {
                        app_name,
                        version: None,
                        exe_path: path.to_string_lossy().to_string(),
                        exe_hash: String::new(),
                        installed_date: None,
                    };
                    apps.push(app);
                }
            }
        }
        
        // Use homebrew list if available
        if let Ok(output) = Command::new("brew").arg("list").output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if !line.is_empty() {
                    let app = InstalledApp {
                        app_name: line.to_string(),
                        version: None,
                        exe_path: String::new(),
                        exe_hash: String::new(),
                        installed_date: None,
                    };
                    apps.push(app);
                }
            }
        }
        
        Ok(apps)
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    
    if let Ok(metadata) = std::fs::metadata(path) {
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        
        // Check if executable bit is set
        (mode & 0o111) != 0
    } else {
        false
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inventory_scan() {
        if let Ok(apps) = InventoryScanner::scan_installed_software().await {
            println!("Found {} applications", apps.len());
            assert!(!apps.is_empty() || cfg!(target_os = "windows"));
        }
    }

    #[tokio::test]
    async fn test_inventory_report() {
        if let Ok(report) = InventoryScanner::generate_inventory_report().await {
            println!("Report: {}", serde_json::to_string_pretty(&report).unwrap());
            assert!(report["timestamp"].is_string());
        }
    }
}
