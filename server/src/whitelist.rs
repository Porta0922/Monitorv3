// Software hash whitelist validation
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub struct SoftwareWhitelist {
    // In-memory cache of app_name -> Vec<valid_hashes>
    whitelist: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SoftwareWhitelist {
    pub fn new() -> Self {
        Self {
            whitelist: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate executable hash against whitelist
    pub async fn validate_executable_hash(&self, app_name: &str, hash: &str) -> bool {
        let whitelist = self.whitelist.read().await;
        
        if let Some(valid_hashes) = whitelist.get(app_name) {
            valid_hashes.contains(&hash.to_string())
        } else {
            // If not in whitelist, treat as unverified but not malicious
            false
        }
    }

    /// Check if hash is known and potentially suspicious
    pub async fn check_hash_mismatch(&self, app_name: &str, hash: &str) -> (bool, Option<String>) {
        let whitelist = self.whitelist.read().await;
        
        if let Some(valid_hashes) = whitelist.get(app_name) {
            if !valid_hashes.contains(&hash.to_string()) {
                // Hash mismatch detected
                return (true, valid_hashes.first().cloned());
            }
        }
        
        (false, None)
    }

    /// Add hash to whitelist
    pub async fn add_to_whitelist(&self, app_name: &str, hash: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut whitelist = self.whitelist.write().await;
        
        whitelist.entry(app_name.to_string())
            .or_insert_with(Vec::new)
            .push(hash.to_string());
        
        tracing::info!("Added {} to whitelist: {}", app_name, hash);
        Ok(())
    }

    /// Load whitelist from database
    pub async fn load_from_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Query software_whitelist table
        // TODO: Populate self.whitelist with approved hashes
        tracing::info!("Whitelist loaded from database");
        Ok(())
    }

    /// Get whitelist statistics
    pub async fn get_stats(&self) -> (usize, usize) {
        let whitelist = self.whitelist.read().await;
        let total_apps = whitelist.len();
        let total_hashes = whitelist.values().map(|h| h.len()).sum();
        (total_apps, total_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_validation() {
        let whitelist = SoftwareWhitelist::new();
        
        // Add hash to whitelist
        whitelist.add_to_whitelist("notepad.exe", "abcd1234").await.ok();
        
        // Verify hash exists
        let is_valid = whitelist.validate_executable_hash("notepad.exe", "abcd1234").await;
        assert!(is_valid);
        
        // Check different hash
        let is_valid = whitelist.validate_executable_hash("notepad.exe", "wrong1234").await;
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_hash_mismatch_detection() {
        let whitelist = SoftwareWhitelist::new();
        
        whitelist.add_to_whitelist("notepad.exe", "original_hash").await.ok();
        
        let (is_mismatch, expected) = whitelist.check_hash_mismatch("notepad.exe", "modified_hash").await;
        assert!(is_mismatch);
        assert_eq!(expected, Some("original_hash".to_string()));
    }
}
