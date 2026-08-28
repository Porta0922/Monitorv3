// API client for the dashboard
import axios from 'axios';
import type { AxiosInstance } from 'axios';
import type {
  Device,
  ActivityLog,
  AppInfo,
  RunningAppInfo,
  USBEvent,
  WifiEvent,
  SecurityAlert,
  SecurityEvent,
  LoginResponse,
  NodeResourceMetric,
  DeviceResourcePeak,
} from '../types';

const BASE_URL = `http://${window.location.hostname}:3000/api`;

function getClientUtcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

class ApiClient {
  private client: AxiosInstance;
  private token: string | null = null;

  constructor() {
    this.client = axios.create({
      baseURL: BASE_URL,
      timeout: 10000,
    });

    // Load token from localStorage
    this.token = localStorage.getItem('auth_token');
    this.updateAuthHeader();

    // Add response interceptor for 401
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.status === 401) {
          this.clearAuth();
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }

  private updateAuthHeader() {
    if (this.token) {
      this.client.defaults.headers.common['Authorization'] = `Bearer ${this.token}`;
    } else {
      delete this.client.defaults.headers.common['Authorization'];
    }
  }

  // Auth
  async login(username: string, password: string): Promise<LoginResponse> {
    const response = await this.client.post<LoginResponse>('/auth/login', {
      username,
      password,
    });
    this.token = response.data.token;
    localStorage.setItem('auth_token', this.token);
    this.updateAuthHeader();
    return response.data;
  }

  logout() {
    this.clearAuth();
  }

  private clearAuth() {
    this.token = null;
    localStorage.removeItem('auth_token');
    this.updateAuthHeader();
  }

  isAuthenticated(): boolean {
    return this.token !== null;
  }

  // Devices
  async getDevices(): Promise<Device[]> {
    const response = await this.client.get<{ devices: Device[] }>('/devices', {
      params: { tz_offset_minutes: getClientUtcOffsetMinutes() },
    });
    return response.data.devices || [];
  }

  async getDevice(deviceId: string): Promise<Device> {
    const response = await this.client.get<Device>(`/devices/${deviceId}`, {
      params: { tz_offset_minutes: getClientUtcOffsetMinutes() },
    });
    return response.data;
  }

  async updateDevice(deviceId: string, nickname: string): Promise<Device> {
    const response = await this.client.patch<Device>(`/devices/${deviceId}`, {
      nickname,
    });
    return response.data;
  }

  async registerDevice(deviceId: string, hostname: string, macAddress: string): Promise<Device> {
    const response = await this.client.post<Device>('/devices/register', {
      device_id: deviceId,
      hostname,
      mac_address: macAddress,
    });
    return response.data;
  }

  // Activity Logs
  async getActivityLogs(
    deviceId?: string,
    options: {
      limit?: number;
      hours?: number;
      from?: string;
      to?: string;
    } = {}
  ): Promise<ActivityLog[]> {
    const params = {
      limit: options.limit ?? 100,
      ...(options.hours ? { hours: options.hours } : {}),
      ...(options.from ? { from: options.from } : {}),
      ...(options.to ? { to: options.to } : {}),
    };
    if (deviceId) {
      const response = await this.client.get<{ logs: ActivityLog[] }>(`/logs/${deviceId}`, {
        params,
      });
      return response.data.logs || [];
    }
    const response = await this.client.get<{ logs: ActivityLog[] }>('/logs', { params });
    return response.data.logs || [];
  }

  async ingestActivityLogs(deviceId: string, events: any[]): Promise<void> {
    await this.client.post('/logs/ingest', {
      device_id: deviceId,
      events,
    });
  }

  // Software Inventory
  async getApps(deviceId?: string): Promise<AppInfo[]> {
    if (deviceId) {
      const response = await this.client.get<{ apps: AppInfo[] }>(`/inventory/apps/${deviceId}`);
      return response.data.apps || [];
    }
    const response = await this.client.get<{ apps: AppInfo[] }>('/inventory/apps');
    return response.data.apps || [];
  }

  async getRunningApps(deviceId: string): Promise<RunningAppInfo[]> {
    const response = await this.client.get<{ apps: RunningAppInfo[] }>(`/inventory/running_apps/${deviceId}`);
    return response.data.apps || [];
  }

  // USB Events
  async getUsbHistory(deviceId?: string, limit = 100, date?: string): Promise<USBEvent[]> {
    const params = {
      limit,
      tz_offset_minutes: getClientUtcOffsetMinutes(),
      ...(date ? { date } : {}),
    };
    if (deviceId) {
      const response = await this.client.get<{ events: USBEvent[] }>(`/usb/${deviceId}`, {
        params,
      });
      return response.data.events || [];
    }
    const response = await this.client.get<{ events: USBEvent[] }>('/usb', { params });
    return response.data.events || [];
  }

  // WiFi Events
  async getWifiHistory(deviceId?: string, limit = 100, date?: string): Promise<WifiEvent[]> {
    const params = {
      limit,
      tz_offset_minutes: getClientUtcOffsetMinutes(),
      ...(date ? { date } : {}),
    };
    if (deviceId) {
      const response = await this.client.get<{ events: WifiEvent[] }>(`/wifi/${deviceId}`, {
        params,
      });
      return response.data.events || [];
    }
    const response = await this.client.get<{ events: WifiEvent[] }>('/wifi', { params });
    return response.data.events || [];
  }

  // Security Alerts
  async getAlerts(severity?: string, resolved = false): Promise<SecurityAlert[]> {
    const params: any = { resolved };
    if (severity) params.severity = severity;
    const response = await this.client.get<{ alerts: SecurityAlert[] }>('/alerts', { params });
    return response.data.alerts || [];
  }

  async resolveAlert(alertId: number): Promise<SecurityAlert> {
    const response = await this.client.patch<{ success?: boolean; alert?: SecurityAlert; error?: string }>(
      `/alerts/${alertId}/resolve`,
      {}
    );

    if (response.data?.success && response.data.alert) {
      return response.data.alert;
    }

    throw new Error(response.data?.error || 'No fue posible resolver la alerta');
  }

  // Security Events (osquery + MITRE ATT&CK)
  async getSecurityEvents(params: {
    deviceId?: string;
    from?: string;
    to?: string;
    hours?: number;
    severity?: string;
    mitreFilter?: string;
    limit?: number;
  } = {}): Promise<SecurityEvent[]> {
    const response = await this.client.get<{ events: SecurityEvent[] }>('/security', {
      params: {
        ...(params.deviceId ? { device_id: params.deviceId } : {}),
        ...(params.from ? { from: params.from } : {}),
        ...(params.to ? { to: params.to } : {}),
        ...(params.hours ? { hours: params.hours } : {}),
        ...(params.severity ? { severity: params.severity } : {}),
        ...(params.mitreFilter ? { mitre_technique: params.mitreFilter } : {}),
        ...(params.limit ? { limit: params.limit } : {}),
      },
    });
    return response.data.events || [];
  }

  async getSecurityEventsByDevice(deviceId: string, params: {
    from?: string;
    to?: string;
    severity?: string;
    mitreFilter?: string;
    limit?: number;
  } = {}): Promise<SecurityEvent[]> {
    const response = await this.client.get<{ events: SecurityEvent[] }>(`/security/${deviceId}`, {
      params: {
        ...(params.from ? { from: params.from } : {}),
        ...(params.to ? { to: params.to } : {}),
        ...(params.severity ? { severity: params.severity } : {}),
        ...(params.mitreFilter ? { mitre_technique: params.mitreFilter } : {}),
        ...(params.limit ? { limit: params.limit } : {}),
      },
    });
    return response.data.events || [];
  }

  async getSecuritySummary(params: {
    deviceId?: string;
    from?: string;
    to?: string;
  } = {}): Promise<{
    total_today: number;
    critical_count: number;
    top_technique: string;
    by_severity_and_technique: Array<{ severity: string; mitre_technique: string; count: number }>;
  }> {
    const response = await this.client.get('/security/summary', {
      params: {
        ...(params.deviceId ? { device_id: params.deviceId } : {}),
        ...(params.from ? { from: params.from } : {}),
        ...(params.to ? { to: params.to } : {}),
      },
    });
    return response.data;
  }

  // Dashboard Overview & Insights
  async getOverview(): Promise<{
    devices_today: number;
    active_time: number;
    idle_time: number;
    idle_pct: string;
    keys_today: number;
    mouse_moves_today: number;
    mouse_clicks_today: number;
  }> {
    const response = await this.client.get<{
      success: boolean;
      data: {
        devices_today: number;
        active_time: number;
        idle_time: number;
        idle_pct: string;
        keys_today: number;
        mouse_moves_today: number;
        mouse_clicks_today: number;
      };
    }>('/overview');
    return response.data.data;
  }

  async getTopApps(): Promise<
    Array<{
      app_name: string;
      total_duration_seconds: number;
      total_duration_hours: string;
    }>
  > {
    const response = await this.client.get<{
      success: boolean;
      data: Array<{
        app_name: string;
        total_duration_seconds: number;
        total_duration_hours: string;
      }>;
    }>('/top_apps');
    return response.data.data || [];
  }

  // Analytics (V1 parity)
  async getHistory(deviceId: string, date?: string): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; history: any[] }>('/history', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    return response.data.history || [];
  }

  async getHistoryHourlyPrograms(deviceId: string, date?: string): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; groups: any[] }>('/history_hourly_programs', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    return response.data.groups || [];
  }

  async getHourly(deviceId: string, date?: string): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; hourly: any[] }>('/hourly', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    return response.data.hourly || [];
  }

  async getAvailableDates(deviceId?: string): Promise<string[]> {
    const response = await this.client.get<{ success: boolean; dates: string[] }>('/available_dates', {
      params: {
        ...(deviceId ? { device_id: deviceId } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    return response.data.dates || [];
  }

  async getActiveVsIdle(days = 7): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; data: any[] }>('/active_vs_idle', {
      params: { days },
    });
    return response.data.data || [];
  }

  async getLiveDevices(options: { liveOnly?: boolean; limit?: number } = {}): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; devices: any[] }>('/live_devices', {
      params: {
        ...(typeof options.liveOnly === 'boolean' ? { live_only: options.liveOnly } : {}),
        ...(options.limit ? { limit: options.limit } : {}),
      },
    });
    return response.data.devices || [];
  }

  async getMetricsSummary(): Promise<any> {
    const response = await this.client.get<{ success: boolean; metrics: any }>('/metrics/summary');
    return response.data;
  }

  async getDeviceResources(deviceId: string, date?: string, limit = 2880): Promise<NodeResourceMetric[]> {
    const response = await this.client.get<{ success: boolean; metrics: NodeResourceMetric[] }>(`/resources/${deviceId}`, {
      params: {
        ...(date ? { date } : {}),
        limit,
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    return response.data.metrics || [];
  }

  async getResourcePeaks(limit = 20): Promise<DeviceResourcePeak[]> {
    const response = await this.client.get<{ success: boolean; peaks: DeviceResourcePeak[] }>('/resources_peaks', {
      params: { limit },
    });
    return response.data.peaks || [];
  }

  async getAuditEvents(limit = 100): Promise<any[]> {
    const response = await this.client.get<{ success: boolean; events: any[] }>('/audit', {
      params: { limit },
    });
    return response.data.events || [];
  }

  async exportCsv(params?: { deviceId?: string; from?: string; to?: string }): Promise<Blob> {
    const response = await this.client.get('/export/csv', {
      params: {
        ...(params?.deviceId ? { device_id: params.deviceId } : {}),
        ...(params?.from ? { from: params.from } : {}),
        ...(params?.to ? { to: params.to } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
      responseType: 'blob',
    });
    return response.data as Blob;
  }

  // Health
  async getHealth(): Promise<{ status: string; version: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }
}

export const apiClient = new ApiClient();
