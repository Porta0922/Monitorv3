// Offline cache using SQLite + AES-GCM encryption
use rusqlite::{Connection, Result as SqliteResult};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::Aead, KeyInit};
use rand::Rng;
use serde_json::Value;
use chrono::Utc;
use uuid::Uuid;

pub struct OfflineCache {
    db_path: String,
    cipher: Aes256Gcm,
}

impl OfflineCache {
    /// Initialize offline cache with encryption
    pub fn new(db_path: &str, encryption_key: &[u8; 32]) -> SqliteResult<Self> {
        // Create database if not exists
        let conn = Connection::open(db_path)?;
        
        // Create cache table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                encrypted_payload BLOB NOT NULL,
                nonce BLOB NOT NULL,
                timestamp TEXT NOT NULL,
                synced INTEGER DEFAULT 0
            )",
            [],
        )?;
        
        conn.close().ok();
        
        // Initialize cipher
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        let cipher = Aes256Gcm::new(key);
        
        Ok(Self {
            db_path: db_path.to_string(),
            cipher,
        })
    }

    /// Save event to offline cache with encryption
    pub async fn save_event(&self, event_type: &str, payload: &Value) -> SqliteResult<String> {
        let conn = Connection::open(&self.db_path)?;
        let event_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        // Generate random nonce
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt payload
        let plaintext = payload.to_string();
        let ciphertext = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Store encrypted data
        conn.execute(
            "INSERT INTO cache_events (id, event_type, encrypted_payload, nonce, timestamp, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            rusqlite::params![&event_id, event_type, ciphertext, nonce_bytes.to_vec(), timestamp],
        )?;

        Ok(event_id)
    }

    /// Retrieve unsynced events from cache (FIFO order)
    pub async fn get_unsynced_events(&self) -> SqliteResult<Vec<(String, String, Value)>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, event_type, encrypted_payload, nonce FROM cache_events 
             WHERE synced = 0 ORDER BY timestamp ASC"
        )?;

        let events = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let event_type: String = row.get(1)?;
            let ciphertext: Vec<u8> = row.get(2)?;
            let nonce_bytes: Vec<u8> = row.get(3)?;

            // Decrypt
            let nonce = Nonce::from_slice(&nonce_bytes);
            let plaintext = self.cipher.decrypt(nonce, ciphertext.as_ref())
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            
            let payload: Value = serde_json::from_slice(&plaintext)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

            Ok((id, event_type, payload))
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Mark events as synced
    pub async fn mark_synced(&self, event_ids: &[String]) -> SqliteResult<()> {
        let conn = Connection::open(&self.db_path)?;
        
        for id in event_ids {
            conn.execute(
                "UPDATE cache_events SET synced = 1 WHERE id = ?1",
                rusqlite::params![id],
            )?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> SqliteResult<(u64, u64)> {
        let conn = Connection::open(&self.db_path)?;
        
        let total: u64 = conn.query_row(
            "SELECT COUNT(*) FROM cache_events",
            [],
            |row| row.get(0),
        )?;

        let unsynced: u64 = conn.query_row(
            "SELECT COUNT(*) FROM cache_events WHERE synced = 0",
            [],
            |row| row.get(0),
        )?;

        Ok((total, unsynced))
    }

    /// Clear old synced events (cleanup)
    pub async fn cleanup_synced(&self, days_old: i64) -> SqliteResult<u64> {
        let conn = Connection::open(&self.db_path)?;
        
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days_old);
        
        let deleted = conn.execute(
            "DELETE FROM cache_events WHERE synced = 1 AND timestamp < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;

        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_encryption() {
        let key: [u8; 32] = [0u8; 32];
        let cache = OfflineCache::new(":memory:", &key)
            .expect("Failed to create cache");

        let payload = json!({
            "device_id": "test-device",
            "app_name": "test.exe"
        });

        let event_id = cache.save_event("activity_log", &payload)
            .await
            .expect("Failed to save event");

        assert!(!event_id.is_empty());

        let (total, unsynced) = cache.get_stats()
            .await
            .expect("Failed to get stats");

        assert_eq!(total, 1);
        assert_eq!(unsynced, 1);
    }
}
