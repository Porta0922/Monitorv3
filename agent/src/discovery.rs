use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfigFile {
    pub api_version: Option<String>,
    pub version: Option<String>,
    pub agent: Option<AgentConfigSection>,
    pub server: Option<ServerConfigSection>,
    pub rabbitmq: Option<RabbitMqConfigSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfigSection {
    pub auth_token: Option<String>,
    pub offline_cache_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfigSection {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RabbitMqConfigSection {
    pub url: Option<String>,
}

/// Results from the discovery process.
/// These are layered: discovery can fill in missing env vars.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryResult {
    pub server_url: Option<String>,
    pub rabbitmq_url: Option<String>,
    pub auth_token: Option<String>,
    pub offline_cache_key: Option<String>,
}

/// Discovers agent configuration from local sources.
///
/// Order:
/// 1. agent-config.json in standard locations
/// 2. Windows Registry (HKLM\SOFTWARE\ActivityMonitor\Agent)
/// 3. DNS SRV lookup (future: _activity-monitor._tcp.<domain>)
///
/// Environment variables take precedence over all (handled by ConfigManager::from_env).
pub fn discover() -> DiscoveryResult {
    let mut result = DiscoveryResult::default();

    // 1. Try config file
    if let Some(file_result) = discover_config_file() {
        merge_config(&mut result, file_result);
    }

    // 2. Try registry (Windows only)
    #[cfg(windows)]
    {
        if let Some(reg_result) = discover_registry() {
            merge_config(&mut result, reg_result);
        }
    }

    // 3. DNS SRV discovery (stub for future)
    // discover_dns_srv(&mut result);

    result
}

fn merge_config(result: &mut DiscoveryResult, source: DiscoveryResult) {
    if result.server_url.is_none() && source.server_url.is_some() {
        result.server_url = source.server_url;
    }
    if result.rabbitmq_url.is_none() && source.rabbitmq_url.is_some() {
        result.rabbitmq_url = source.rabbitmq_url;
    }
    if result.auth_token.is_none() && source.auth_token.is_some() {
        result.auth_token = source.auth_token;
    }
    if result.offline_cache_key.is_none() && source.offline_cache_key.is_some() {
        result.offline_cache_key = source.offline_cache_key;
    }
}

/// Find and parse agent-config.json from standard locations.
fn discover_config_file() -> Option<DiscoveryResult> {
    let candidates = config_file_candidates();
    
    for path in candidates {
        if path.exists() {
            tracing::info!("[Discovery] Found config file: {}", path.display());
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<AgentConfigFile>(&content) {
                        Ok(config) => {
                            let mut result = DiscoveryResult::default();
                            if let Some(agent) = &config.agent {
                                result.auth_token = agent.auth_token.clone();
                                result.offline_cache_key = agent.offline_cache_key.clone();
                            }
                            if let Some(server) = &config.server {
                                result.server_url = server.url.clone();
                            }
                            if let Some(rabbitmq) = &config.rabbitmq {
                                result.rabbitmq_url = rabbitmq.url.clone();
                            }
                            if result.auth_token.is_some()
                                || result.server_url.is_some()
                                || result.rabbitmq_url.is_some()
                                || result.offline_cache_key.is_some()
                            {
                                tracing::info!("[Discovery] Loaded config from: {}", path.display());
                                return Some(result);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[Discovery] Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("[Discovery] Failed to read {}: {}", path.display(), e);
                }
            }
        }
    }

    None
}

/// Returns candidate paths for agent-config.json, in priority order.
fn config_file_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            paths.push(parent.join("agent-config.json"));
        }
    }

    // 2. Platform-specific config directories
    if cfg!(windows) {
        paths.push(PathBuf::from(r"C:\ProgramData\ActivityMonitor\Config\agent-config.json"));
        paths.push(PathBuf::from(r"C:\ProgramData\ActivityMonitor\agent-config.json"));
    } else {
        paths.push(PathBuf::from("/etc/activity-monitor/agent-config.json"));
        paths.push(PathBuf::from("/etc/activity-monitor/config.json"));
    }

    // 3. Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("agent-config.json"));
    }

    paths
}

/// Windows Registry discovery.
#[cfg(windows)]
fn discover_registry() -> Option<DiscoveryResult> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = r"SOFTWARE\ActivityMonitor\Agent";

    match hklm.open_subkey(key_path) {
        Ok(key) => {
            let mut result = DiscoveryResult::default();

            if let Ok(val) = key.get_value::<String, _>("AuthToken") {
                if !val.is_empty() && val != "change-me-in-production" {
                    result.auth_token = Some(val);
                }
            }
            if let Ok(val) = key.get_value::<String, _>("ServerUrl") {
                if !val.is_empty() {
                    result.server_url = Some(val);
                }
            }
            if let Ok(val) = key.get_value::<String, _>("RabbitMqUrl") {
                if !val.is_empty() {
                    result.rabbitmq_url = Some(val);
                }
            }
            if let Ok(val) = key.get_value::<String, _>("OfflineCacheKey") {
                if !val.is_empty() && val != "replace-with-32-byte-cache-key!!" {
                    result.offline_cache_key = Some(val);
                }
            }

            if result.auth_token.is_some()
                || result.server_url.is_some()
                || result.rabbitmq_url.is_some()
            {
                tracing::info!("[Discovery] Loaded config from Windows Registry");
                return Some(result);
            }

            None
        }
        Err(_) => None,
    }
}

/// Applies discovery results to environment variables (if not already set).
/// This should be called before ConfigManager::from_env().
pub fn apply_to_env(discovery: &DiscoveryResult) {
    if let Some(url) = &discovery.server_url {
        if std::env::var("AGENT_SERVER_URL").is_err() {
            std::env::set_var("AGENT_SERVER_URL", url);
            tracing::info!("[Discovery] Set AGENT_SERVER_URL from discovery");
        }
    }
    if let Some(url) = &discovery.rabbitmq_url {
        if std::env::var("RABBITMQ_URL").is_err() {
            std::env::set_var("RABBITMQ_URL", url);
            tracing::info!("[Discovery] Set RABBITMQ_URL from discovery");
        }
    }
    if let Some(token) = &discovery.auth_token {
        if std::env::var("AGENT_AUTH_TOKEN").is_err() {
            std::env::set_var("AGENT_AUTH_TOKEN", token);
            tracing::info!("[Discovery] Set AGENT_AUTH_TOKEN from discovery");
        }
    }
    if let Some(key) = &discovery.offline_cache_key {
        if std::env::var("AGENT_OFFLINE_CACHE_KEY").is_err() {
            std::env::set_var("AGENT_OFFLINE_CACHE_KEY", key);
            tracing::info!("[Discovery] Set AGENT_OFFLINE_CACHE_KEY from discovery");
        }
    }
}
