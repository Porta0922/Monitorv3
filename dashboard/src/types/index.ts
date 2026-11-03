// Types for the dashboard

export interface Device {
  group_name?: string;
  version?: string | null;
  desired_version?: string | null;
  agent_health?: { status?: string; stale: boolean; observed_at: string; monitoring_paused?: boolean } | null;
  agent_health_error?: boolean;
  device_id: string;
  hostname: string;
  nickname?: string;
  mac_address: string;
  last_seen: string;
  online: boolean;
  stale?: boolean;
  status?: 'online' | 'offline';
  created_at: string;
  active_time_today_seconds?: number;
  idle_time_today_seconds?: number;
  keys_today?: number;
  mouse_moves_today?: number;
  mouse_clicks_today?: number;
}

export interface ActivityLog {
  timestamp: string;
  device_id: string;
  app_name: string;
  window_title: string;
  duration_seconds: number;
}

export interface AppInfo {
  id: number;
  device_id: string;
  app_name: string;
  version?: string;
  exe_hash: string;
  verified: boolean;
  last_detected: string;
}

export interface RunningAppInfo {
  id: string;
  device_id: string;
  app_name: string;
  primary_title: string;
  window_count: number;
  exe_path?: string;
  exe_hash?: string;
  updated_at: string;
  principal?: string | null;
  session_id?: string | null;
  source?: 'session' | 'legacy';
  stale?: boolean;
}

export interface USBEvent {
  timestamp: string;
  device_id: string;
  action: 'IN' | 'OUT';
  hardware_id: string;
  device_name: string;
  serial_number: string;
  volume_label?: string;
}

export interface WifiEvent {
  timestamp: string;
  device_id: string;
  interface_name: string;
  state: string;
  ssid?: string;
  bssid?: string;
  signal_percent?: number;
}

export interface NodeResourceMetric {
  timestamp: string;
  cpu_percent: number;
  memory_used_mb: number;
  memory_percent: number;
  top_process_name?: string;
  top_process_cpu_percent?: number;
  top_process_memory_mb?: number;
}

export interface DeviceResourcePeak {
  device_id: string;
  peak_cpu_percent: number;
  peak_memory_percent: number;
  last_cpu_percent: number;
  last_memory_percent: number;
  top_process_name?: string;
  top_process_cpu_percent?: number;
  top_process_memory_mb?: number;
  last_seen: string;
}

export interface SecurityAlert {
  id: number;
  device_id: string;
  alert_type: string;
  app_name: string;
  exe_hash: string;
  description: string;
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  resolved: boolean;
  created_at: string;
}

export interface SecurityEvent {
  id: number;
  timestamp: string;
  device_id: string;
  query_name: string;
  query_pack?: string;
  mitre_technique?: string;
  severity: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
  raw_data: Record<string, unknown>;
  created_at: string;
}

export interface User {
  id: number;
  username: string;
  email: string;
  is_admin: boolean;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  username: string;
  role: 'viewer'|'operator'|'admin';
  csrf_token: string;
  expires_in: number;
}

export interface DailyMetric {
  day: string;
  resource_samples: number;
  input_samples: number;
  input_available_samples:number;
  input_unavailable_samples:number;
  input_unknown_samples:number;
  cpu_average: number | null;
  cpu_peak: number | null;
  memory_average: number | null;
  memory_peak: number | null;
  memory_used_mb_average: number | null;
  active_seconds: number | null;
  idle_seconds: number | null;
  keys_count: number | null;
  mouse_moves_count: number | null;
  clicks_count: number | null;
  source: 'detail' | 'summary' | 'mixed';
}

export interface DailyMetricsResponse {
  device_id: string;
  from: string;
  to: string;
  timezone: 'UTC';
  average_basis: 'sample';
  days: DailyMetric[];
}

export interface MaintenanceStatus {
  worker_settings?: Record<string,string>;
  relations?: { schema_name: string; relation_name: string; total_bytes: number; heap_bytes: number; index_bytes: number; estimated_rows: number | null }[];
  relations_truncated?: boolean;
  relation_limit?: number;
  observed_at: string;
  database_size_bytes: number;
  metrics: { last_success_at: string } | null;
  history: Array<{
    job: string;
    retention_days: number;
    batch_limit: number;
    last_attempt_at: string;
    last_success_at: string | null;
    last_failure_at: string | null;
    last_error: string | null;
    last_rows_processed: number;
    pending_rows_lower_bound: number | null;
    backlog_truncated: boolean | null;
    oldest_pending_at: string | null;
  }>;
}

export interface SystemMetrics {
  schema_version: 1;
  hardware?: { logical_cpu_count: number; cpu_model: string | null; os_version: string | null; kernel_version: string | null };
  cpu_percent: number | null;
  cpu_state: string;
  memory_used_bytes: number | null;
  memory_total_bytes: number | null;
  memory_percent: number | null;
  memory_state: string;
  uptime_seconds: number | null;
  disks_state: string;
  disks: { mount: string; total_bytes: number; available_bytes: number; used_percent: number | null }[];
  network_state: string;
  network: { name: string; receive_bytes_per_second: number | null; transmit_bytes_per_second: number | null; rate_state: string }[];
  capabilities: AgentCapabilities;
}
export interface AgentCapabilities {
  input_diagnostic?: {state:string;last_error?:string|null;observed_at?:string};
  mode?: string; os?: string; architecture?: string; desktop?: string;
  input_available?: boolean; desktop_capture_eligible?: boolean;
}
export interface AgentHealth {
  policy_fingerprint?: string | null;
  sessions?: { truncated?: boolean; relay_enabled: boolean; producer_mode: string; instances: {
    principal?: string | null; instance_id: string; received_at: string; stale: boolean; monitoring_paused: boolean;
    input_available: boolean; pending_events: number | null; policy_fingerprint: string;
    tasks: {name: string; running: boolean; capture?: {state:string;last_error:string|null;observed_at:string|null}|null}[];
  }[] } | null;
  collectors?: { name: string; state: string; last_error?: string | null; capture?: { state: string; observed_at?: string | null; last_error?: string | null } }[] | null;
  status?: string;
  monitoring_paused?: boolean;
  capabilities?: AgentCapabilities;
  version?: string;
  cache_unsynced_events?: number;
  config_version?: number;
  disabled_monitors?: string[];
  enabled_monitors?: string[];
  tasks?: { name: string; running: boolean; restart_count: number; last_error?: string | null }[];
}
export interface TelemetrySample<T> { observed_at: string; received_at: string; stale: boolean; data: T }
export interface DeviceTelemetry {
  device_id: string; queried_at: string;
  resources: TelemetrySample<SystemMetrics> | null;
  health: TelemetrySample<AgentHealth> | null;
}

export interface InventoryStateRow {
  id?: number; scope: string; source_key: string; app_name?: string; version?: string | null; changed_at?: string;
  action?: 'added' | 'updated' | 'removed'; timestamp?: string;
  before_value?: { app_name: string; version: string | null } | null;
  after_value?: { app_name: string; version: string | null } | null;
}
export interface InventoryStatePage {
  items: InventoryStateRow[]; total: number; limit: number; offset: number;
  sources: { scope: string; observed_at: string; last_complete_at: string | null; last_error: string | null; item_count: number | null; stale: boolean }[];
}
