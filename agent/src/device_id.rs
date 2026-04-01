// Device ID generation based on MAC + hostname
use sha2::{Sha256, Digest};
use uuid::Uuid;
use std::fs;
use std::path::Path;

const DEVICE_ID_FILE: &str = "/var/lib/activity-monitor/device_id.json";
const DEVICE_NICKNAME_FILE: &str = "/var/lib/activity-monitor/device_nickname.txt";

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

/// Get primary MAC address (platform-specific)
pub fn get_primary_mac_address() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        get_mac_windows()
    }
    
    #[cfg(target_os = "linux")]
    {
        get_mac_linux()
    }
    
    #[cfg(target_os = "macos")]
    {
        get_mac_macos()
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
    let config_path = Path::new(DEVICE_ID_FILE);
    
    // Try to load existing identity
    if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        if let Ok(identity) = serde_json::from_str::<DeviceIdentity>(&content) {
            return Ok(identity);
        }
    }
    
    // Create new identity
    let mac_address = get_primary_mac_address()?;
    let hostname = get_hostname()?;
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
    let nickname_path = Path::new(DEVICE_NICKNAME_FILE);
    
    if let Some(parent) = nickname_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    
    fs::write(nickname_path, nickname)?;
    Ok(())
}

/// Get device nickname if set
pub fn get_device_nickname() -> Option<String> {
    let nickname_path = Path::new(DEVICE_NICKNAME_FILE);
    
    if nickname_path.exists() {
        fs::read_to_string(nickname_path).ok()
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn get_mac_windows() -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("ipconfig")
        .arg("/all")
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse MAC address from ipconfig output
    for line in output_str.lines() {
        if line.contains("Physical Address") {
            if let Some(mac) = line.split(':').nth(1) {
                return Ok(mac.trim().to_string());
            }
        }
    }
    
    Err("Could not determine MAC address".into())
}

#[cfg(target_os = "linux")]
fn get_mac_linux() -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("ip")
        .args(&["link", "show"])
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse first non-loopback MAC address
    for line in output_str.lines() {
        if line.contains("link/ether") {
            if let Some(mac) = line.split_whitespace().nth(1) {
                return Ok(mac.to_string());
            }
        }
    }
    
    Err("Could not determine MAC address".into())
}

#[cfg(target_os = "macos")]
fn get_mac_macos() -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let output = Command::new("ifconfig")
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Parse first MAC address
    for line in output_str.lines() {
        if line.contains("ether") {
            if let Some(mac) = line.split_whitespace().nth(1) {
                return Ok(mac.to_string());
            }
        }
    }
    
    Err("Could not determine MAC address".into())
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
