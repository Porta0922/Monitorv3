// Types for the dashboard

export interface Device {
  device_id: string;
  hostname: string;
  nickname?: string;
  mac_address: string;
  last_seen: string;
  online: boolean;
  created_at: string;
  active_time_today_seconds?: number;
  idle_time_today_seconds?: number;
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

export interface USBEvent {
  timestamp: string;
  device_id: string;
  action: 'IN' | 'OUT';
  hardware_id: string;
  device_name: string;
  serial_number: string;
  volume_label?: string;
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
