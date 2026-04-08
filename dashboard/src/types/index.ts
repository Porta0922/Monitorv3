// Types for the dashboard

export interface Device {
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
  token: string;
  expires_in: number;
  token_type: string;
}
