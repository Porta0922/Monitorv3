// osquery runner: executes MITRE ATT&CK-mapped queries via osqueryi and returns findings.
// Silent no-op when osqueryi is not installed.
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A security finding produced by one osquery SQL query.
#[derive(Debug, Clone)]
pub struct OsqueryFinding {
    pub query_name: String,
    pub query_pack: String,
    pub mitre_technique: Option<String>,
    pub severity: String,
    /// Up to MAX_ROWS rows returned by osquery (JSON objects).
    pub raw_data: Vec<Value>,
    /// SHA-256 of query_name + row content — used for server-side dedup.
    pub event_fingerprint: Option<String>,
}

// ─── Query catalogue ────────────────────────────────────────────────────────

struct QueryDef {
    name: &'static str,
    pack: &'static str,
    mitre_technique: Option<&'static str>,
    severity: &'static str,
    sql: &'static str,
}

/// Max rows to include per finding to keep payloads bounded.
const MAX_ROWS: usize = 50;

static QUERIES: &[QueryDef] = &[
    // T1053.005 – hidden scheduled tasks (Windows + macOS)
    QueryDef {
        name: "scheduled_tasks_hidden",
        pack: "attck_t1053",
        mitre_technique: Some("T1053.005"),
        severity: "HIGH",
        sql: "SELECT name, action, path, enabled, hidden \
              FROM scheduled_tasks WHERE hidden = 1;",
    },
    // T1547.001 – persistence via Registry Run keys
    QueryDef {
        name: "autorun_registry_keys",
        pack: "attck_t1547",
        mitre_technique: Some("T1547.001"),
        severity: "MEDIUM",
        sql: "SELECT name, path, source, status, username \
              FROM autoexec WHERE source LIKE 'Registry%';",
    },
    // T1059.001 – PowerShell with encoded-command or bypass flag
    QueryDef {
        name: "powershell_encoded_commands",
        pack: "attck_t1059",
        mitre_technique: Some("T1059.001"),
        severity: "HIGH",
        sql: "SELECT pid, name, path, cmdline \
              FROM processes \
              WHERE (name LIKE '%powershell%' OR name LIKE '%pwsh%') \
                AND (cmdline LIKE '%-enc%' \
                  OR cmdline LIKE '%-EncodedCommand%' \
                  OR cmdline LIKE '%bypass%');",
    },
    // T1036 – masquerading: unsigned binaries running from Windows system paths
    QueryDef {
        name: "unsigned_system_path_processes",
        pack: "attck_t1036",
        mitre_technique: Some("T1036"),
        severity: "MEDIUM",
        sql: "SELECT p.pid, p.name, p.path, a.signed \
              FROM processes p \
              JOIN authenticode a ON p.path = a.path \
              WHERE a.signed = '0' AND p.path LIKE 'C:\\Windows\\%';",
    },
    // T1105 – ingress tool transfer: executables running from Temp
    QueryDef {
        name: "executable_in_temp_paths",
        pack: "attck_t1105",
        mitre_technique: Some("T1105"),
        severity: "HIGH",
        sql: "SELECT pid, name, path, cmdline \
              FROM processes \
              WHERE (path LIKE '%\\Temp\\%' OR path LIKE '%AppData%\\Temp%') \
                AND (name LIKE '%.exe' OR name LIKE '%.bat' OR name LIKE '%.ps1');",
    },
    // T1021 – remote services: non-loopback listening ports outside expected set
    QueryDef {
        name: "unusual_listening_ports",
        pack: "attck_t1021",
        mitre_technique: Some("T1021"),
        severity: "MEDIUM",
        sql: "SELECT pid, port, protocol, address \
              FROM listening_ports \
              WHERE port > 1024 \
                AND port NOT IN (3389, 5900, 8080, 8443, 9000, 49152) \
                AND address NOT IN ('127.0.0.1', '::1', '0.0.0.0');",
    },
    // T1547.009 – shortcut / startup item persistence
    QueryDef {
        name: "startup_items",
        pack: "attck_t1547",
        mitre_technique: Some("T1547.009"),
        severity: "LOW",
        sql: "SELECT name, path, args, type, source, status, username \
              FROM startup_items;",
    },
    // T1057 – process discovery: short-lived cmd.exe spawned by unusual parents
    QueryDef {
        name: "cmd_spawned_by_unusual_parent",
        pack: "attck_t1059",
        mitre_technique: Some("T1059.003"),
        severity: "MEDIUM",
        sql: "SELECT p.pid, p.name, p.cmdline, p.path, pp.name AS parent_name \
              FROM processes p \
              JOIN processes pp ON p.parent = pp.pid \
              WHERE p.name IN ('cmd.exe', 'wscript.exe', 'cscript.exe') \
                AND pp.name NOT IN ('explorer.exe', 'svchost.exe', 'cmd.exe', 'pwsh.exe', 'WindowsTerminal.exe');",
    },
];

// ─── Runner ─────────────────────────────────────────────────────────────────

pub struct OsqueryRunner;

impl OsqueryRunner {
    /// Run all registered queries against the local osqueryi binary.
    /// Returns an empty vec (without logging errors) when osqueryi is absent.
    pub async fn scan_once() -> Vec<OsqueryFinding> {
        let Some(osqueryi) = Self::find_osqueryi() else {
            tracing::debug!("osqueryi not found – skipping security scan");
            return vec![];
        };

        tracing::debug!("osquery security scan starting ({} queries)", QUERIES.len());

        let mut findings = Vec::new();

        for query in QUERIES {
            match Self::run_query(&osqueryi, query).await {
                Ok(rows) if !rows.is_empty() => {
                    tracing::info!(
                        "🛡️  osquery '{}': {} row(s) detected (MITRE: {})",
                        query.name,
                        rows.len(),
                        query.mitre_technique.unwrap_or("-")
                    );
                    let capped: Vec<Value> = rows.into_iter().take(MAX_ROWS).collect();
                    let fp = Self::fingerprint(query.name, &capped);
                    findings.push(OsqueryFinding {
                        query_name: query.name.to_string(),
                        query_pack: query.pack.to_string(),
                        mitre_technique: query.mitre_technique.map(|s| s.to_string()),
                        severity: query.severity.to_string(),
                        raw_data: capped,
                        event_fingerprint: Some(fp),
                    });
                }
                Ok(_) => {
                    tracing::debug!("osquery '{}': no results", query.name);
                }
                Err(e) => {
                    // Table may not exist on this OS — log at debug, not warn.
                    tracing::debug!("osquery '{}' error: {}", query.name, e);
                }
            }
        }

        tracing::debug!(
            "osquery scan complete: {}/{} queries produced findings",
            findings.len(),
            QUERIES.len()
        );

        findings
    }

    // ── osqueryi location ────────────────────────────────────────────────────

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

        // Last resort: check PATH via `where` (Windows) / `which` (Unix)
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

    // ── Query execution ──────────────────────────────────────────────────────

    async fn run_query(
        osqueryi: &str,
        query: &QueryDef,
    ) -> Result<Vec<Value>, String> {
        let output = tokio::process::Command::new(osqueryi)
            .args(["--json", query.sql])
            // Prevent osqueryi from inheriting the agent's console on Windows.
            .creation_flags_if_windows(0x0800_0000) // CREATE_NO_WINDOW
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // osqueryi exits 0 even on unsupported table; stderr contains the hint.
        if !stderr.is_empty() {
            tracing::trace!("osquery '{}' stderr: {}", query.name, stderr.trim());
        }

        let rows: Vec<Value> = serde_json::from_str(stdout.trim())
            .map_err(|e| format!("JSON parse error: {e}"))?;

        Ok(rows)
    }

    // ── Fingerprint ──────────────────────────────────────────────────────────

    fn fingerprint(query_name: &str, rows: &[Value]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query_name.as_bytes());
        for row in rows {
            hasher.update(row.to_string().as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

// ─── Platform helper trait ───────────────────────────────────────────────────

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
