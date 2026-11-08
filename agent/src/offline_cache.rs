// Offline cache using SQLite + AES-GCM encryption with WAL and hardware-bound secure key derivation
use rusqlite::{Connection, Result as SqliteResult};
use aes_gcm::{KeyInit, Aes256Gcm, aead::Aead};
use aes_gcm::{Key, Nonce};
use rand::Rng;
use serde_json::Value;
use chrono::Utc;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use sha2::Digest;

pub const REPLAY_BATCH_SIZE: usize = 256;

pub struct OfflineCache {
    conn: Arc<Mutex<Connection>>,
    cipher: Aes256Gcm,
}

impl OfflineCache {
    /// Initialize offline cache with WAL connection reuse and encryption
    pub fn new(db_path: &str, encryption_key: &[u8; 32]) -> SqliteResult<Self> {
        let conn = Self::init_db(db_path)?;
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        Ok(Self { conn: Arc::new(Mutex::new(conn)), cipher: Aes256Gcm::new(key) })
    }

    fn init_db(db_path: &str) -> SqliteResult<Connection> {
        let conn = Connection::open(db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        
        // Durable acknowledgement boundary for local IPC and publisher retries.
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA temp_store = MEMORY;
        ")?;

        // Comprobación de integridad rápida de la base de datos
        if db_path != ":memory:" {
            let integrity = conn.query_row("PRAGMA integrity_check(1);", [], |row| row.get::<_, String>(0))?;
            if integrity != "ok" { return Err(rusqlite::Error::InvalidQuery); }
        }

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
        
        conn.execute("CREATE INDEX IF NOT EXISTS idx_cache_events_pending ON cache_events(synced, timestamp)", [])?;
        Ok(conn)
    }

    /// Save event to offline cache with encryption
    pub async fn save_event(&self, event_type: &str, payload: &Value) -> SqliteResult<String> {
        let event_id = payload.get("event_id").and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok()).unwrap_or_else(Uuid::new_v4).to_string();
        let timestamp = Utc::now().to_rfc3339();

        // Generate random nonce
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt payload
        let plaintext = payload.to_string();
        let ciphertext = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        // Reutilizar la conexión persistente con bloqueo thread-safe
        let conn = self.conn.lock().unwrap();
        
        // Retries of an already accepted envelope are idempotent, including synced rows.
        let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM cache_events WHERE id = ?1)", [&event_id], |r| r.get(0))?;
        if exists { return Ok(event_id); }
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM cache_events WHERE synced = 0", [], |r| r.get(0))?;
        if count >= 20000 {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
                Some("Offline queue is full; pending events were preserved".into())));
        }

        conn.execute(
            "INSERT INTO cache_events (id, event_type, encrypted_payload, nonce, timestamp, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            rusqlite::params![&event_id, event_type, ciphertext, nonce_bytes.to_vec(), timestamp],
        )?;

        Ok(event_id)
    }

    /// Retrieve unsynced events from cache (FIFO order)
    pub async fn get_unsynced_events(&self) -> SqliteResult<Vec<(String, String, Value)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_type, encrypted_payload, nonce FROM cache_events 
             WHERE synced = 0 ORDER BY timestamp ASC, rowid ASC LIMIT ?1"
        )?;

        let events = stmt.query_map([REPLAY_BATCH_SIZE as i64], |row| {
            let id: String = row.get(0)?;
            let event_type: String = row.get(1)?;
            let ciphertext: Vec<u8> = row.get(2)?;
            let nonce_bytes: Vec<u8> = row.get(3)?;

            // Decrypt
            if nonce_bytes.len() != 12 { return Err(rusqlite::Error::InvalidQuery); }
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
        let mut conn = self.conn.lock().unwrap();
        let transaction = conn.transaction()?;
        for id in event_ids {
            transaction.execute(
                "UPDATE cache_events SET synced = 1 WHERE id = ?1",
                rusqlite::params![id],
            )?;
        }

        transaction.commit()
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> SqliteResult<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        
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

    /// Get count of unsynced events
    pub async fn get_unsynced_event_count(&self) -> SqliteResult<u64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM cache_events WHERE synced = 0",
            [],
            |row| row.get(0),
        )
    }

    /// Clear all unsynced events
    pub async fn clear_unsynced_events(&self) -> SqliteResult<u64> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM cache_events WHERE synced = 0",
            [],
        )?;
        Ok(deleted as u64)
    }

    /// Clear old synced events (cleanup)
    pub async fn cleanup_synced(&self, days_old: i64) -> SqliteResult<u64> {
        let conn = self.conn.lock().unwrap();
        
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days_old);
        
        let deleted = conn.execute(
            "DELETE FROM cache_events WHERE synced = 1 AND timestamp < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;

        Ok(deleted as u64)
    }
}

/// Obtener un identificador único persistente del hardware del sistema operativo
fn get_os_machine_id() -> String {
    #[cfg(target_os = "windows")]
    {
        use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(crypt) = hklm.open_subkey(r"SOFTWARE\Microsoft\Cryptography") {
            if let Ok(guid) = crypt.get_value::<String, _>("MachineGuid") {
                return guid.trim().to_string();
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/etc/machine-id") {
            return content.trim().to_string();
        }
        if let Ok(content) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
            return content.trim().to_string();
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ioreg").args(&["-rd1", "-c", "IOPlatformExpertDevice"]).output() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(uuid) = line.split('"').nth(3) {
                        return uuid.trim().to_string();
                    }
                }
            }
        }
    }
    "default-hardware-encryption-id-value".to_string()
}

/// Generar y resolver una clave de cifrado local robusta y única, vinculada criptográficamente al hardware
pub fn resolve_secure_key(env_key: Option<&str>, device_id: &Uuid) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    
    // 1. Agregar la clave base configurada en el entorno (si existe)
    if let Some(k) = env_key {
        hasher.update(k.as_bytes());
    } else {
        hasher.update(b"dev-cache-key-change-in-production-");
    }
    
    // 2. Vincular el UUID estable del dispositivo
    hasher.update(device_id.as_bytes());
    
    // 3. Vincular el Machine ID único a nivel de sistema operativo
    let machine_id = get_os_machine_id();
    hasher.update(machine_id.as_bytes());
    
    // 4. Agregar sal criptográfica estática del agente
    hasher.update(b"ActivityMonitor-Secure-Local-Salt-2026");
    
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash[..32]);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_cache_encryption() {
        let db_path = "test_cache_encryption.db";
        let _ = std::fs::remove_file(db_path);

        let key: [u8; 32] = [0u8; 32];
        let cache = OfflineCache::new(db_path, &key)
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

        // Liberar explícitamente la conexión SQLite para poder borrar el archivo en Windows
        drop(cache);

        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}

#[cfg(test)]
mod durability_tests {
    use super::*;
    #[tokio::test]
    async fn confirmation_batch_rolls_back_if_any_write_fails() {
        let cache = OfflineCache::new(":memory:", &[7; 32]).unwrap();
        let first = cache.save_event("heartbeat", &serde_json::json!({"event_id":Uuid::new_v4()})).await.unwrap();
        let second = cache.save_event("heartbeat", &serde_json::json!({"event_id":Uuid::new_v4()})).await.unwrap();
        cache.conn.lock().unwrap().execute_batch(&format!(
            "CREATE TRIGGER reject_confirmation BEFORE UPDATE ON cache_events WHEN NEW.id='{second}' BEGIN SELECT RAISE(ABORT,'injected failure'); END;"
        )).unwrap();
        assert!(cache.mark_synced(&[first.clone(), second.clone()]).await.is_err());
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 2);
        cache.conn.lock().unwrap().execute_batch("DROP TRIGGER reject_confirmation;").unwrap();
        cache.mark_synced(&[first, second]).await.unwrap();
        assert_eq!(cache.get_unsynced_event_count().await.unwrap(), 0);
    }
    #[tokio::test]
    async fn reopen_preserves_ids_wrong_key_fails_and_full_queue_keeps_pending() {
        let dir = std::env::temp_dir().join(format!("monitor-cache-{}", Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("queue.sqlite"); let path = path.to_str().unwrap();
        let key = [7;32];
        let id = Uuid::new_v4().to_string();
        let event = serde_json::json!({"event_id":id,"payload":{"cpu":42}});
        let cache = OfflineCache::new(path, &key).unwrap();
        assert_eq!(cache.save_event("heartbeat", &event).await.unwrap(), id);
        drop(cache);
        let wrong = OfflineCache::new(path, &[8;32]).unwrap();
        assert!(wrong.get_unsynced_events().await.is_err()); drop(wrong);
        let cache = OfflineCache::new(path, &key).unwrap();
        assert_eq!(cache.get_unsynced_events().await.unwrap()[0].2, event);
        cache.save_event("heartbeat", &event).await.unwrap();
        assert_eq!(cache.get_stats().await.unwrap(), (1,1));
        cache.conn.lock().unwrap().execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<19999)
            INSERT INTO cache_events SELECT 'filler-'||x, 'test', x'00', x'00', '2026-01-01', 0 FROM n;").unwrap();
        let next = serde_json::json!({"event_id":Uuid::new_v4()});
        assert!(cache.save_event("heartbeat", &next).await.is_err());
        assert_eq!(cache.get_stats().await.unwrap(), (20000,20000));
        assert_eq!(cache.save_event("heartbeat", &event).await.unwrap(), id);
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn corrupted_cache_is_not_replaced() {
        let path = std::env::temp_dir().join(format!("monitor-corrupt-{}.sqlite", Uuid::new_v4()));
        let bytes = b"this is not a sqlite database";
        std::fs::write(&path, bytes).unwrap();
        assert!(OfflineCache::new(path.to_str().unwrap(), &[0;32]).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
    }
}
