// API client for the dashboard
import axios from 'axios';
import type { AxiosInstance } from 'axios';
import type { Device, ActivityLog, AppInfo, USBEvent, SecurityAlert, LoginResponse } from '../types';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

class ApiClient {
  private client: AxiosInstance;
  private token: string | null = null;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
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
    const response = await this.client.get<{ devices: Device[] }>('/devices');
    return response.data.devices || [];
  }

  async getDevice(deviceId: string): Promise<Device> {
    const response = await this.client.get<Device>(`/devices/${deviceId}`);
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
  async getActivityLogs(deviceId?: string, limit = 100): Promise<ActivityLog[]> {
    const params = { limit };
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

  // USB Events
  async getUsbHistory(deviceId?: string, limit = 100): Promise<USBEvent[]> {
    const params = { limit };
    if (deviceId) {
      const response = await this.client.get<{ events: USBEvent[] }>(`/usb/${deviceId}`, {
        params,
      });
      return response.data.events || [];
    }
    const response = await this.client.get<{ events: USBEvent[] }>('/usb', { params });
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
    const response = await this.client.patch<SecurityAlert>(`/alerts/${alertId}`, {
      resolved: true,
    });
    return response.data;
  }

  // Health
  async getHealth(): Promise<{ status: string; version: string }> {
    const response = await this.client.get('/health');
    return response.data;
  }
}

export const apiClient = new ApiClient();
