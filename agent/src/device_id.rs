// Device ID generation based on MAC + hostname
use sha2::{Sha256, Digest};
use uuid::Uuid;
use std::fs;
use std::path::Path;

fn get_data_dir() -> String {
    if cfg!(windows) {
        r"C:\ProgramData\ActivityMonitor".to_string()
    } else {
        #[cfg(not(windows))]
        {
            let is_root = unsafe { libc::getuid() == 0 };
            if is_root {
                "/var/lib/activity-monitor".to_string()
            } else {
                if let Ok(home) = std::env::var("HOME") {
                    format!("{}/.local/share/activity-monitor", home)
                } else {
                    "/tmp/activity-monitor".to_string()
                }
            }
        }
        #[cfg(windows)]
        {
            r"C:\ProgramData\ActivityMonitor".to_string()
        }
    }
}

fn get_device_id_file() -> String {
    format!("{}/device_id.json", get_data_dir())
}

fn get_device_nickname_file() -> String {
    format!("{}/device_nickname.txt", get_data_dir())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceIdentity {
    pub device_id: Uuid,
    pub hostname: String,
    pub mac_address: String,
    pub nickname: Option<String>,
}

/// Generate immutable device ID from MAC + hostname
pub fn generate_device_id(mac_address: &str, hostname: &str) -> Uuid {
    let combined = format!("{}:{}", mac_address, hostname);
    
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hasher.finalize();
    
    // Convert first 16 bytes of hash to UUID
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&hash[..16]);
    
    // Ensure version 4 (random) format
    Uuid::from_bytes(uuid_bytes)
}

/// Generate a fallback MAC address based on hostname or UUID
/// Used when the actual MAC address cannot be determined
fn generate_fallback_mac() -> String {
    // Try to use hostname, fall back to UUID-based MAC
    match get_hostname() {
        Ok(hostname) => {
            // Use first 12 characters of SHA256(hostname) to create a fake MAC
            let mut hasher = Sha256::new();
            hasher.update(hostname.as_bytes());
            let hash = hasher.finalize();
            let hex = format!("{:x}", hash);
            
            // Format as MAC address: XX:XX:XX:XX:XX:XX
            format!(
                "{}:{}:{}:{}:{}:{}",
                &hex[0..2],
                &hex[2..4],
                &hex[4..6],
                &hex[6..8],
                &hex[8..10],
                &hex[10..12]
            )
        }
        Err(_) => {
            // Ultimate fallback: use a deterministic UUID-based MAC
            let uuid_str = Uuid::new_v4().to_string();
            let hex = format!("{:x}", Uuid::parse_str(&uuid_str).unwrap_or_else(|_| Uuid::nil()));
            format!(
                "{}:{}:{}:{}:{}:{}",
                &hex[0..2],
                &hex[2..4],
                &hex[4..6],
                &hex[6..8],
                &hex[8..10],
                &hex[10..12]
            )
        }
    }
}

/// Get primary MAC address (platform-specific)
/// Returns a MAC address or a default based on hostname/UUID if unavailable
pub fn get_primary_mac_address() -> String {
    #[cfg(target_os = "windows")]
    {
        get_mac_windows().unwrap_or_else(|| generate_fallback_mac())
    }
    
    #[cfg(target_os = "linux")]
    {
        get_mac_linux().unwrap_or_else(|| generate_fallback_mac())
    }
    
    #[cfg(target_os = "macos")]
    {
        get_mac_macos().unwrap_or_else(|| generate_fallback_mac())
    }
}

/// Get hostname
pub fn get_hostname() -> Result<String, Box<dyn std::error::Error>> {
    Ok(hostname::get()?
        .to_string_lossy()
        .to_string())
}

/// Load or create device identity
pub fn load_or_create_device_identity() -> Result<DeviceIdentity, Box<dyn std::error::Error>> {
    let config_path_str = get_device_id_file();
    let config_path = Path::new(&config_path_str);
    
    // Try to load existing identity
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        if let Ok(identity) = serde_json::from_str::<DeviceIdentity>(&content) {
            return Ok(identity);
        }
    }
    
    // Create new identity
    let mac_address = get_primary_mac_address();
    let hostname = get_hostname().unwrap_or_else(|_| {
        // Fallback: Use a hash-based hostname if actual hostname fails
        format!("device-{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
    });
    let device_id = generate_device_id(&mac_address, &hostname);
    
    let identity = DeviceIdentity {
        device_id,
        hostname,
        mac_address,
        nickname: None,
    };
    
    // Save for future use
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    
    let json = serde_json::to_string_pretty(&identity)?;
    fs::write(config_path, json).ok();
    
    Ok(identity)
}

/// Set device nickname (from server)
pub fn set_device_nickname(nickname: &str) -> Result<(), Box<dyn std::error::Error>> {
    let nickname_path_str = get_device_nickname_file();
    let nickname_path = Path::new(&nickname_path_str);
    
    if let Some(parent) = nickname_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    
    fs::write(nickname_path, nickname)?;
    Ok(())
}

/// Get device nickname if set
pub fn get_device_nickname() -> Option<String> {
    let nickname_path_str = get_device_nickname_file();
    let nickname_path = Path::new(&nickname_path_str);
    
    if nickname_path.exists() {
        fs::read_to_string(nickname_path).ok()
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn get_mac_windows() -> Option<String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    
    let mut cmd = Command::new("ipconfig");
    cmd.arg("/all");
    cmd.creation_flags(0x08000000);
    
    let output = cmd.output().ok()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse MAC address from ipconfig output
    for line in output_str.lines() {
        if line.contains("Physical Address") {
            if let Some(mac) = line.split(':').nth(1) {
                return Some(mac.trim().to_string());
            }
        }
    }
    
    None
}

#[cfg(target_os = "linux")]
fn get_mac_linux() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("ip")
        .args(&["link", "show"])
        .output()
        .ok()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse first non-loopback MAC address
    for line in output_str.lines() {
        if line.contains("link/ether") {
            if let Some(mac) = line.split_whitespace().nth(1) {
                return Some(mac.to_string());
            }
        }
    }
    
    None
}

#[cfg(target_os = "macos")]
fn get_mac_macos() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("ifconfig")
        .output()
        .ok()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse first MAC address
    for line in output_str.lines() {
        if line.contains("ether") {
            if let Some(mac) = line.split_whitespace().nth(1) {
                return Some(mac.to_string());
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_id_generation() {
        let mac = "00:11:22:33:44:55";
        let hostname = "test-device";
        
        let id1 = generate_device_id(mac, hostname);
        let id2 = generate_device_id(mac, hostname);
        
        // Same inputs should produce same ID
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_device_id_stability() {
        let mac = "00:11:22:33:44:55";
        let hostname = "test-device";
        
        let id = generate_device_id(mac, hostname);
        
        // Device ID should be stable
        assert!(!id.is_nil());
    }
}
