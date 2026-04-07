// osquery runner with per-query scheduling to avoid overloading endpoints.
use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct OsqueryFinding {
    pub query_name: String,
    pub query_pack: String,
    pub mitre_technique: Option<String>,
    pub severity: String,
    pub raw_data: Vec<Value>,
    pub event_fingerprint: Option<String>,
}

struct QueryDef {
    name: &'static str,
    pack: &'static str,
    mitre_technique: Option<&'static str>,
    severity: &'static str,
    interval_seconds: i64,
    max_rows: usize,
    sql: &'static str,
}

static QUERIES: &[QueryDef] = &[
    // Fast/high-risk checks (every 5 minutes)
    QueryDef {
        name: "powershell_encoded_commands",
        pack: "custom_attack_surface",
        mitre_technique: Some("T1059.001"),
        severity: "HIGH",
        interval_seconds: 300,
        max_rows: 50,
        sql: "SELECT pid, name, path, cmdline \
              FROM processes \
              WHERE (name LIKE '%powershell%' OR name LIKE '%pwsh%') \
                AND (cmdline LIKE '%-enc%' \
                  OR cmdline LIKE '%-EncodedCommand%' \
                  OR cmdline LIKE '%bypass%');",
    },
    QueryDef {
        name: "powershell_download_cradles",
        pack: "custom_attack_surface",
        mitre_technique: Some("T1105"),
        severity: "HIGH",
        interval_seconds: 300,
        max_rows: 50,
        sql: "SELECT pid, name, cmdline \
              FROM processes \
              WHERE (name LIKE '%powershell%' OR name LIKE '%pwsh%') \
                AND (cmdline LIKE '%Invoke-WebRequest%' \
                  OR cmdline LIKE '%iwr %' \
                  OR cmdline LIKE '%DownloadString%' \
                  OR cmdline LIKE '%Net.WebClient%');",
    },
    QueryDef {
        name: "suspicious_script_hosts",
        pack: "custom_attack_surface",
        mitre_technique: Some("T1059.005"),
        severity: "MEDIUM",
        interval_seconds: 300,
        max_rows: 60,
        sql: "SELECT pid, name, cmdline, path \
              FROM processes \
              WHERE name IN ('wscript.exe', 'cscript.exe', 'mshta.exe') \
                AND (cmdline LIKE '%http%' \
                  OR cmdline LIKE '%\\\\%'
                  OR cmdline LIKE '%.js%'
                  OR cmdline LIKE '%.vbs%');",
    },
    // Medium checks (every 10 minutes)
    QueryDef {
        name: "unusual_listening_ports",
        pack: "custom_network",
        mitre_technique: Some("T1021"),
        severity: "MEDIUM",
        interval_seconds: 600,
        max_rows: 100,
        sql: "SELECT pid, port, protocol, address \
              FROM listening_ports \
              WHERE port > 1024 \
                AND port NOT IN (3389, 5900, 8080, 8443, 9000, 49152) \
                AND address NOT IN ('127.0.0.1', '::1', '0.0.0.0');",
    },
    QueryDef {
        name: "executable_in_temp_paths",
        pack: "custom_execution_paths",
        mitre_technique: Some("T1105"),
        severity: "HIGH",
        interval_seconds: 600,
        max_rows: 80,
        sql: "SELECT pid, name, path, cmdline \
              FROM processes \
              WHERE (path LIKE '%\\Temp\\%' OR path LIKE '%AppData%\\Temp%') \
                AND (name LIKE '%.exe' OR name LIKE '%.bat' OR name LIKE '%.ps1');",
    },
    QueryDef {
        name: "lolbins_with_remote_content",
        pack: "custom_lolbins",
        mitre_technique: Some("T1218"),
        severity: "HIGH",
        interval_seconds: 600,
        max_rows: 50,
        sql: "SELECT pid, name, cmdline, path \
              FROM processes \
              WHERE name IN ('regsvr32.exe', 'rundll32.exe', 'mshta.exe', 'certutil.exe', 'bitsadmin.exe') \
                AND (cmdline LIKE '%http%' OR cmdline LIKE '%https%');",
    },
    // Slower baseline checks (every 20-30 minutes)
    QueryDef {
        name: "scheduled_tasks_hidden",
        pack: "custom_persistence",
        mitre_technique: Some("T1053.005"),
        severity: "HIGH",
        interval_seconds: 1200,
        max_rows: 200,
        sql: "SELECT name, action, path, enabled, hidden \
              FROM scheduled_tasks WHERE hidden = 1;",
    },
    QueryDef {
        name: "autorun_registry_keys",
        pack: "custom_persistence",
        mitre_technique: Some("T1547.001"),
        severity: "MEDIUM",
        interval_seconds: 1200,
        max_rows: 200,
        sql: "SELECT name, path, source, status, username \
              FROM autoexec WHERE source LIKE 'Registry%';",
    },
    QueryDef {
        name: "cmd_spawned_by_unusual_parent",
        pack: "custom_process_chain",
        mitre_technique: Some("T1059.003"),
        severity: "MEDIUM",
        interval_seconds: 1200,
        max_rows: 80,
        sql: "SELECT p.pid, p.name, p.cmdline, p.path, pp.name AS parent_name \
              FROM processes p \
              JOIN processes pp ON p.parent = pp.pid \
              WHERE p.name IN ('cmd.exe', 'wscript.exe', 'cscript.exe') \
                AND pp.name NOT IN ('explorer.exe', 'svchost.exe', 'cmd.exe', 'pwsh.exe', 'WindowsTerminal.exe');",
    },
    QueryDef {
        name: "unsigned_system_path_processes",
        pack: "custom_masquerading",
        mitre_technique: Some("T1036"),
        severity: "MEDIUM",
        interval_seconds: 1800,
        max_rows: 120,
        sql: "SELECT p.pid, p.name, p.path, a.signed \
              FROM processes p \
              JOIN authenticode a ON p.path = a.path \
              WHERE a.signed = '0' AND p.path LIKE 'C:\\Windows\\%';",
    },
    QueryDef {
        name: "startup_items_persistence",
        pack: "custom_persistence",
        mitre_technique: Some("T1547.009"),
        severity: "LOW",
        interval_seconds: 1800,
        max_rows: 200,
        sql: "SELECT name, path, args, type, source, status, username \
              FROM startup_items;",
    },
];

pub struct OsqueryRunner {
    last_run: HashMap<&'static str, DateTime<Utc>>,
}

impl OsqueryRunner {
    pub fn new() -> Self {
        Self {
            last_run: HashMap::new(),
        }
    }

    pub async fn scan_due(&mut self) -> Vec<OsqueryFinding> {
        let Some(osqueryi) = Self::find_osqueryi() else {
            tracing::debug!("osqueryi not found, skipping security scan");
            return vec![];
        };

        let now = Utc::now();
        let mut findings = Vec::new();

        for query in QUERIES {
            if !self.is_due(query, now) {
                continue;
            }

            self.last_run.insert(query.name, now);

            match Self::run_query(&osqueryi, query).await {
                Ok(rows) if !rows.is_empty() => {
                    let capped: Vec<Value> = rows.into_iter().take(query.max_rows).collect();
                    let fp = Self::fingerprint(query.name, &capped);
                    findings.push(OsqueryFinding {
                        query_name: query.name.to_string(),
                        query_pack: query.pack.to_string(),
                        mitre_technique: query.mitre_technique.map(str::to_string),
                        severity: query.severity.to_string(),
                        raw_data: capped,
                        event_fingerprint: Some(fp),
                    });
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!("osquery '{}' error: {}", query.name, e);
                }
            }
        }

        findings
    }

    fn is_due(&self, query: &QueryDef, now: DateTime<Utc>) -> bool {
        match self.last_run.get(query.name) {
            None => true,
            Some(last) => now.signed_duration_since(*last) >= Duration::seconds(query.interval_seconds.max(60)),
        }
    }

    fn find_osqueryi() -> Option<String> {
        #[cfg(target_os = "windows")]
        let candidates: &[&str] = &[
            r"C:\Program Files\osquery\osqueryi.exe",
            r"C:\Program Files (x86)\osquery\osqueryi.exe",
            "osqueryi.exe",
            "osqueryi",
        ];

        #[cfg(target_os = "linux")]
        let candidates: &[&str] = &["/usr/bin/osqueryi", "/usr/local/bin/osqueryi", "osqueryi"];

        #[cfg(target_os = "macos")]
        let candidates: &[&str] = &[
            "/usr/local/bin/osqueryi",
            "/opt/homebrew/bin/osqueryi",
            "osqueryi",
        ];

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let candidates: &[&str] = &["osqueryi"];

        for candidate in candidates {
            if std::path::Path::new(candidate).exists() {
                return Some((*candidate).to_string());
            }
        }

        #[cfg(target_os = "windows")]
        let locator = "where";
        #[cfg(not(target_os = "windows"))]
        let locator = "which";

        if let Ok(out) = std::process::Command::new(locator).arg("osqueryi").output() {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path.lines().next().unwrap_or("").to_string());
                }
            }
        }

        None
    }

    async fn run_query(osqueryi: &str, query: &QueryDef) -> Result<Vec<Value>, String> {
        let output = tokio::process::Command::new(osqueryi)
            .args(["--json", query.sql])
            .creation_flags_if_windows(0x0800_0000)
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stderr.trim().is_empty() {
            tracing::trace!("osquery '{}' stderr: {}", query.name, stderr.trim());
        }

        let rows: Vec<Value> = serde_json::from_str(stdout.trim())
            .map_err(|e| format!("JSON parse error: {e}"))?;

        Ok(rows)
    }

    fn fingerprint(query_name: &str, rows: &[Value]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query_name.as_bytes());
        for row in rows {
            hasher.update(row.to_string().as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

trait CommandExt {
    fn creation_flags_if_windows(&mut self, flags: u32) -> &mut Self;
}

impl CommandExt for tokio::process::Command {
    fn creation_flags_if_windows(&mut self, flags: u32) -> &mut Self {
        #[cfg(target_os = "windows")]
        self.creation_flags(flags);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = flags;
        }
        self
    }
}
