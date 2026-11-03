//! Atomic replacement of small configuration snapshots in the same directory.
use std::{io::{self, Write}, path::Path};
pub fn write_json(path: &Path, value: &impl serde::Serialize) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::other("Missing parent directory"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".snapshot-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new(); options.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&temporary)?;
        file.write_all(&serde_json::to_vec(value).map_err(io::Error::other)?)?;
        file.sync_all()?; drop(file);
        #[cfg(windows)] {
            use std::os::windows::ffi::OsStrExt;
            use winapi::um::winbase::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
            let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(not(windows))] {
            std::fs::rename(&temporary, path)?;
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&temporary); }
    result
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    struct CannotSerialize;
    impl serde::Serialize for CannotSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("injected serialization failure"))
        }
    }
    #[test]
    fn failed_replacement_preserves_snapshot_and_cleans_temporary_file() {
        let dir = std::env::temp_dir().join(format!("monitor-snapshot-{}", uuid::Uuid::new_v4()));
        let path = dir.join("identity.json");
        write_json(&path, &serde_json::json!({"id":"original"})).unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(write_json(&path, &CannotSerialize).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        write_json(&path, &serde_json::json!({"id":"replacement"})).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(after["id"], "replacement");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
