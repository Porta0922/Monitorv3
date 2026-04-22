use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct UsbCopyFinding {
    pub drive_letter: String,
    pub file_path: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub modified_utc: DateTime<Utc>,
    pub fingerprint: String,
}

#[derive(Default)]
pub struct UsbFileCopyMonitor {
    seen: HashMap<String, DateTime<Utc>>,
    dedupe_ttl_seconds: i64,
}

#[derive(Debug, Deserialize)]
struct RawFinding {
    drive: Option<String>,
    path: Option<String>,
    name: Option<String>,
    size: Option<i64>,
    modified_utc: Option<String>,
}

impl UsbFileCopyMonitor {
    pub fn new(dedupe_ttl_seconds: i64) -> Self {
        Self {
            seen: HashMap::new(),
            dedupe_ttl_seconds: dedupe_ttl_seconds.max(60),
        }
    }

    pub async fn scan_recent_writes(
        &mut self,
        window_seconds: i64,
        max_files_per_drive: i64,
    ) -> Result<Vec<UsbCopyFinding>, String> {
        self.prune_seen();

        #[cfg(target_os = "windows")]
        {
            self.scan_windows_recent_writes(window_seconds, max_files_per_drive)
                .await
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = window_seconds;
            let _ = max_files_per_drive;
            Ok(Vec::new())
        }
    }

    fn prune_seen(&mut self) {
        let cutoff = Utc::now() - Duration::seconds(self.dedupe_ttl_seconds);
        self.seen.retain(|_, ts| *ts >= cutoff);
    }

    fn fingerprint(raw: &RawFinding) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw.drive.clone().unwrap_or_default().as_bytes());
        hasher.update(raw.path.clone().unwrap_or_default().as_bytes());
        hasher.update(raw.name.clone().unwrap_or_default().as_bytes());
        hasher.update(raw.size.unwrap_or(0).to_string().as_bytes());
        hasher.update(raw.modified_utc.clone().unwrap_or_default().as_bytes());
        hex::encode(hasher.finalize())
    }

    #[cfg(target_os = "windows")]
    async fn scan_windows_recent_writes(
        &mut self,
        window_seconds: i64,
        max_files_per_drive: i64,
    ) -> Result<Vec<UsbCopyFinding>, String> {
        let lookback = window_seconds.clamp(30, 3600);
        let max_files = max_files_per_drive.clamp(5, 100);

        let script = format!(
                        "$cutoffUtc=(Get-Date).ToUniversalTime().AddSeconds(-{lookback}); \
             $max={max_files}; \
                         $usbDrives=@(); \
                         try {{ \
                             $usbDisks=Get-CimInstance Win32_DiskDrive | Where-Object {{ $_.PNPDeviceID -match '^USBSTOR\\\\' -or $_.InterfaceType -eq 'USB' }}; \
                             foreach($disk in $usbDisks){{ \
                                 $parts=Get-CimAssociatedInstance -InputObject $disk -Association Win32_DiskDriveToDiskPartition -ErrorAction SilentlyContinue; \
                                 foreach($part in $parts){{ \
                                     $letters=Get-CimAssociatedInstance -InputObject $part -Association Win32_LogicalDiskToPartition -ErrorAction SilentlyContinue; \
                                     foreach($letter in $letters){{ \
                                         if($letter.DeviceID){{ $usbDrives += $letter.DeviceID }} \
                                     }} \
                                 }} \
                             }} \
                         }} catch {{}}; \
                         $removableDrives=Get-CimInstance Win32_LogicalDisk | Where-Object {{ $_.DriveType -eq 2 -and $_.DeviceID }} | Select-Object -ExpandProperty DeviceID; \
                         $drives=@($usbDrives + $removableDrives | Sort-Object -Unique); \
             $results=@(); \
                         foreach($driveId in $drives){{ \
                             $root=\"$driveId\\\"; \
               if(Test-Path -LiteralPath $root){{ \
                 try {{ \
                                     $items=Get-ChildItem -LiteralPath $root -File -Recurse -Force -ErrorAction SilentlyContinue | \
                                         ForEach-Object {{ \
                                             $writeUtc=$_.LastWriteTime.ToUniversalTime(); \
                                             $createUtc=$_.CreationTime.ToUniversalTime(); \
                                             $eventUtc=if($createUtc -gt $writeUtc){{ $createUtc }} else {{ $writeUtc }}; \
                                             if($eventUtc -ge $cutoffUtc){{ \
                                                 [pscustomobject]@{{ \
                                                     Drive=$driveId; \
                                                     FullName=$_.FullName; \
                                                     Name=$_.Name; \
                                                     Length=[int64]$_.Length; \
                                                     EventUtc=$eventUtc; \
                                                 }} \
                                             }} \
                                         }} | \
                                         Sort-Object EventUtc -Descending | \
                                         Select-Object -First $max; \
                   foreach($f in $items){{ \
                     $results += [pscustomobject]@{{ \
                                             drive=$f.Drive; \
                                             path=$f.FullName; \
                                             name=$f.Name; \
                                             size=[int64]$f.Length; \
                                             modified_utc=$f.EventUtc.ToString('o') \
                     }}; \
                   }} \
                 }} catch {{}} \
               }} \
             }}; \
             $results | ConvertTo-Json -Compress"
        );

        let mut cmd = tokio::process::Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-Command", &script]);
        
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000);
        }
        
        let output = cmd.output().await.map_err(|e| e.to_string())?;

        if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        return Err(if stderr.is_empty() {
                                "powershell usb copy scan failed".to_string()
                        } else {
                                format!("powershell usb copy scan failed: {}", stderr)
                        });
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if raw.is_empty() || raw == "null" {
            return Ok(Vec::new());
        }

        let parsed: Vec<RawFinding> = if raw.starts_with('[') {
            serde_json::from_str(&raw).map_err(|e| e.to_string())?
        } else {
            vec![serde_json::from_str(&raw).map_err(|e| e.to_string())?]
        };

        let mut findings = Vec::new();

        for item in parsed {
            let fp = Self::fingerprint(&item);
            if self.seen.contains_key(&fp) {
                continue;
            }

            let modified_utc = item
                .modified_utc
                .as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let finding = UsbCopyFinding {
                drive_letter: item.drive.unwrap_or_else(|| "UNKNOWN".to_string()),
                file_path: item.path.unwrap_or_else(|| "UNKNOWN".to_string()),
                file_name: item.name.unwrap_or_else(|| "UNKNOWN".to_string()),
                size_bytes: item.size.unwrap_or(0),
                modified_utc,
                fingerprint: fp.clone(),
            };

            self.seen.insert(fp, Utc::now());
            findings.push(finding);
        }

        Ok(findings)
    }
}
