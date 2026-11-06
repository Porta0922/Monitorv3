import { groupHourlyPrograms, duration } from './desktop';
import type { WindowRow } from './desktop';
// API client for the dashboard
import axios from 'axios';
import type { AxiosInstance } from 'axios';
import type {
  Device,
  DeviceTelemetry,
  InventoryStatePage,
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
  DailyMetricsResponse,
  MaintenanceStatus,
} from '../types';

const BASE_URL = '/api';

function getClientUtcOffsetMinutes(): number {
  return 0; // All dashboard reporting days are UTC, including historical days.
}

export interface Investigation {
  id: string; device_id: string; title: string; reason: string;
  event_from: string; event_to: string; created_at: string;
  released_at: string | null; release_reason: string | null; created_by: string | null; released_by: string | null;
}
export interface DeadLetterStatus { queues: {queue: string; ready: number | null; available: boolean}[]; observed_at: string; replay_days: number }
export interface StoragePage {items: {schema_name: string; relation_name: string; total_bytes: number; heap_bytes: number; index_bytes: number; estimated_rows: number | null}[]; has_more: boolean; observed_at: string}
export interface EquipmentPage {devices: Device[]; total: number; page: number; page_size: number; counts: {total:number; online:number; offline:number; stale:number}; groups:string[]}
export interface AgentPolicy { [key:string]: number | boolean | string[]; monitoring_paused:boolean; enabled_monitors:string[]; disabled_monitors:string[] }
export interface PolicyRecord {fingerprint:string;observed:{fingerprint:string|null;observed_at:string;stale:boolean}|null;scope:string; revision:number; override:boolean; policy:AgentPolicy; updated_at:string|null}
export interface VersionSummary {versions:{version:string;devices:number}[]; targeted:number;matching:number;different:number}
export interface AccountDirectory { source: 'managed_file'|'bootstrap'; accounts: {username:string;role:'viewer'|'operator'|'admin';disabled:boolean;mfa_enabled:boolean}[] }
export interface SessionCounters {available_samples:number;unavailable_samples:number;unknown_samples:number;active_person_seconds:number|null;idle_person_seconds:number|null;keys_count:number|null;mouse_moves_count:number|null;clicks_count:number|null}
export interface SessionActivityPage {day:string;timezone:string;total:number;totals:SessionCounters;items:(SessionCounters&{principal:string;session_id:string;first_seen:string;last_seen:string;events:number})[]}
export interface AgentCredential {credential_id:string;device_id:string;created_at:string;expires_at:string;revoked_at:string|null}
class ApiClient {
  async getAgentCredentials(device:string,signal?:AbortSignal):Promise<{credentials:AgentCredential[];broker_managed:boolean}> {return (await this.client.get<{credentials:AgentCredential[];broker_managed:boolean}>(`/agent-credentials/${encodeURIComponent(device)}`,{signal})).data;}
  async enrollAgent(device:string):Promise<{code:string;device_id:string;expires_at:string}> {return (await this.client.post(`/agent-credentials/enrollment/${encodeURIComponent(device)}`,{},{timeout:60000})).data;}
  async revokeAgentCredential(device:string,id:string):Promise<boolean> {return (await this.client.delete(`/agent-credentials/${encodeURIComponent(device)}/${encodeURIComponent(id)}`,{timeout:60000})).status===202;}

  async exportSessionActivity(device:string,day:string):Promise<Blob>{return (await this.client.get(`/devices/${encodeURIComponent(device)}/sessions/export`,{params:{day},responseType:'blob'})).data;}
  async getSessionActivity(device:string,day:string,offset:number,signal?:AbortSignal):Promise<SessionActivityPage> {return (await this.client.get<SessionActivityPage>(`/devices/${encodeURIComponent(device)}/sessions`,{params:{day,offset},signal})).data;}

  async getAccounts(signal?:AbortSignal):Promise<AccountDirectory> {return (await this.client.get<AccountDirectory>('/administration/accounts',{signal})).data;}

  getRole(): 'viewer'|'operator'|'admin' {return this.role;}
  async getEquipmentPage(query: string, signal?:AbortSignal):Promise<EquipmentPage> { return (await this.client.get<EquipmentPage>(`/devices/search?${query}`,{signal})).data; }
  async updateEquipment(id:string, nickname:string, group_name:string, desired_version:string) {return (await this.client.patch(`/devices/${encodeURIComponent(id)}`,{nickname,group_name,desired_version})).data;}
  async getPolicy(scope:string,signal?:AbortSignal):Promise<PolicyRecord> {return (await this.client.get<PolicyRecord>(`/administration/policy/${encodeURIComponent(scope)}`,{signal})).data;}
  async savePolicy(scope:string,revision:number,policy:AgentPolicy|null,reason:string) {return (await this.client.put(`/administration/policy/${encodeURIComponent(scope)}`,{revision,policy,reason})).data;}
  async getVersions(signal?:AbortSignal):Promise<VersionSummary> {return (await this.client.get<VersionSummary>('/administration/versions',{signal})).data;}

  async getStorage(mode: string, offset: number, signal?: AbortSignal) {
    return (await this.client.get<StoragePage>('/maintenance/storage', {params: {mode, offset}, signal})).data;
  }

  async getDeadLetters(signal?: AbortSignal) {
    return (await this.client.get<DeadLetterStatus>('/maintenance/dead-letters', {signal, timeout: 20000})).data;
  }
  async replayDeadLetters(queue: string, limit: number, reason: string) {
    return (await this.client.post<{acknowledged: number; result: {blocked: string | null}; audit_completion_recorded: boolean}>(`/maintenance/dead-letters/${encodeURIComponent(queue)}/replay`, {limit, reason}, {timeout: 30000})).data;
  }

  async getInvestigations(deviceId: string, offset: number, signal?: AbortSignal) {
    return (await this.client.get<{items: Investigation[]; has_more: boolean}>('/security/investigations', {params: {device_id: deviceId || undefined, offset, limit: 20}, signal})).data;
  }
  async createInvestigation(body: Pick<Investigation, 'device_id' | 'title' | 'reason' | 'event_from' | 'event_to'>) {
    return (await this.client.post<Investigation>('/security/investigations', body)).data;
  }
  async releaseInvestigation(id: string, reason: string) {
    return (await this.client.post<Investigation>(`/security/investigations/${encodeURIComponent(id)}/release`, {reason})).data;
  }

  async getInventoryState(deviceId: string, mode: 'current' | 'changes', q: string, offset: number, signal?: AbortSignal): Promise<InventoryStatePage> {
    return (await this.client.get<InventoryStatePage>(`/inventory/${mode}/${encodeURIComponent(deviceId)}`, { params: { q, offset, limit: 50 }, signal })).data;
  }

  async getDeviceTelemetry(deviceId: string, signal?: AbortSignal): Promise<DeviceTelemetry> {
    return (await this.client.get<DeviceTelemetry>(`/devices/${encodeURIComponent(deviceId)}/telemetry`, { signal })).data;
  }

  async getMaintenanceStatus(signal?: AbortSignal): Promise<MaintenanceStatus> {
    return (await this.client.get<MaintenanceStatus>('/maintenance/status', { signal })).data;
  }

  private client: AxiosInstance;
  private role: 'viewer'|'operator'|'admin' = 'viewer';
  private authenticated = false;
  private initialized = false;
  private restoration: Promise<void>|null = null;
  private listeners = new Set<()=>void>();
  constructor() {
    this.client = axios.create({baseURL: BASE_URL,timeout:10000,withCredentials:true});
    localStorage.removeItem('auth_token');
    this.client.defaults.headers.common['X-Monitor-Client']='dashboard';
    this.client.interceptors.response.use(response=>{const role=response.headers['x-monitor-role'];if(['viewer','operator','admin'].includes(role)&&this.role!==role){this.role=role;this.changed();}return response;},error=>{
      if(error.response?.status===401){this.clearAuth();if(!error.config?.url?.startsWith('/auth/'))window.location.href='/login';}
      if(error.response?.status===403&&!error.config?.url?.startsWith('/auth/'))window.dispatchEvent(new Event('monitor:forbidden'));
      if(error.response?.status===403&&!error.config?.url?.startsWith('/auth/'))void this.client.get<LoginResponse>('/auth/session').then(r=>this.acceptSession(r.data)).catch(()=>{});
      return Promise.reject(error);
    });
  }
  subscribe(listener:()=>void){this.listeners.add(listener);return ()=>{this.listeners.delete(listener);};}
  private changed(){this.listeners.forEach(listener=>listener());}
  isInitialized(){return this.initialized;}
  private acceptSession(data:LoginResponse){this.authenticated=true;this.role=data.role;this.client.defaults.headers.common['X-CSRF-Token']=data.csrf_token;this.initialized=true;this.changed();}
  async restoreSession():Promise<void>{
    if(this.initialized)return;
    if(!this.restoration)this.restoration=this.client.get<LoginResponse>('/auth/session').then(response=>this.acceptSession(response.data)).catch(()=>this.clearAuth()).finally(()=>{this.initialized=true;this.changed();});
    return this.restoration;
  }
  async login(username:string,password:string,otp=''):Promise<LoginResponse>{
    await this.restoreSession();
    const response=await this.client.post<LoginResponse>('/auth/login',{username,password,otp});this.acceptSession(response.data);return response.data;
  }
  async logout(){await this.client.post('/auth/logout',{});this.clearAuth();}
  private clearAuth(){this.authenticated=false;this.role='viewer';delete this.client.defaults.headers.common['X-CSRF-Token'];localStorage.removeItem('auth_token');this.changed();}
  isAuthenticated(){return this.authenticated;}
  async getAuthSecurity():Promise<{failed_logins_15m:number;denied_requests_15m?:number;csrf_rejections_15m?:number;limited_accounts_24h:number;events:{actor:string;action:string;created_at:string}[]}>{return (await this.client.get('/auth/security')).data;}
  async getHumanSessions():Promise<{session_id:string;created_at:string;last_seen:string;expires_at:string;current:boolean}[]>{return (await this.client.get('/auth/sessions')).data.sessions;}
  async revokeHumanSession(id:string){await this.client.delete(`/auth/sessions/${encodeURIComponent(id)}`);}
  async revokeMySessions(){await this.client.post('/auth/revoke-all',{});this.clearAuth();}
  async revokeUserSessions(username:string){await this.client.delete(`/auth/users/${encodeURIComponent(username)}/sessions`);}

  async getDailyMetrics(deviceId: string, from: string, to: string, signal?: AbortSignal): Promise<DailyMetricsResponse> {
    const response = await this.client.get<DailyMetricsResponse>(`/resources/${deviceId}/daily`, {
      params: { from, to }, signal,
    });
    return response.data;
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

  async sendAgentCommand(deviceId: string, command: string, payload?: Record<string, unknown>): Promise<{ success: boolean; command_id?: string; error?: string }> {
    const response = await this.client.post<{ success: boolean; command_id?: string; error?: string }>('/agent/commands', {
      device_id: deviceId,
      command,
      payload: payload ?? {},
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

  async ingestActivityLogs(deviceId: string, events: unknown[]): Promise<void> {
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
    const params: { resolved: boolean; severity?: string } = { resolved };
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
    active_time: number | null;
    idle_time: number | null;
    idle_pct: number | null;
    keys_today: number | null;
    mouse_moves_today: number | null;
    mouse_clicks_today: number | null;
  }> {
    const response = await this.client.get<{
      success: boolean;
      data: {
        devices_today: number;
        active_time: number | null;
        idle_time: number | null;
        idle_pct: number | null;
        keys_today: number | null;
        mouse_moves_today: number | null;
        mouse_clicks_today: number | null;
      };
    }>('/overview');
    if (!response.data.success) throw new Error('Overview unavailable');
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
  async getHistory(deviceId: string, date?: string): Promise<unknown[]> {
    const response = await this.client.get<{ success: boolean; history: Omit<WindowRow,'hour'>[] }>('/history', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    if (!response.data.success || !Array.isArray(response.data.history)) throw new Error('Activity history unavailable');
    return response.data.history.map(row=>({app:row.app_name,title:row.window_title,seconds:row.total_seconds,duration:duration(row.total_seconds),intervals:row.count,is_idle:false}));
  }

  async getHistoryHourlyPrograms(deviceId: string, date?: string) {
    const response = await this.client.get<{ success: boolean; groups: WindowRow[] }>('/history_hourly_programs', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    if (!response.data.success || !Array.isArray(response.data.groups)) throw new Error('Hourly programs unavailable');
    return groupHourlyPrograms(response.data.groups);
  }

  async getHourly(deviceId: string, date?: string): Promise<unknown[]> {
    const response = await this.client.get<{ success: boolean; hourly: {hour:number;active_seconds:number;idle_seconds:number}[] }>('/hourly', {
      params: {
        device_id: deviceId,
        ...(date ? { date } : {}),
        tz_offset_minutes: getClientUtcOffsetMinutes(),
      },
    });
    if (!response.data.success) throw new Error('Hourly data unavailable');
    return (response.data.hourly || []).map(row=>({...row,label:`${String(row.hour).padStart(2,'0')}:00`}));
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

  async getActiveVsIdle(days = 7): Promise<unknown[]> {
    const response = await this.client.get<{ success: boolean; data: unknown[] }>('/active_vs_idle', {
      params: { days },
    });
    return response.data.data || [];
  }

  async getLiveDevices(options: { liveOnly?: boolean; limit?: number } = {}): Promise<unknown[]> {
    const response = await this.client.get<{ success: boolean; devices: {device_id:string;app_name:string;window_title:string;duration_seconds:number;timestamp:string}[] }>('/live_devices', {
      params: {
        ...(typeof options.liveOnly === 'boolean' ? { live_only: options.liveOnly } : {}),
        ...(options.limit ? { limit: options.limit } : {}),
      },
    });
    if(!response.data.success) throw new Error('Recent activity unavailable');
    return (response.data.devices || []).map(row=>{const age=Math.max(0,Math.floor((Date.now()-Date.parse(row.timestamp))/1000));return {device_id:row.device_id,app:row.app_name,title:row.window_title,last_seen:row.timestamp,ago_sec:age,is_live:age<=90,is_stale:age>90,is_idle:false,duration:duration(row.duration_seconds)};});
  }

  async getMetricsSummary(): Promise<{ success: boolean; metrics: unknown }> {
    const response = await this.client.get<{ success: boolean; metrics: unknown }>('/metrics/summary');
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

  async getAuditEvents(limit = 100): Promise<unknown[]> {
    const response = await this.client.get<{ success: boolean; events: unknown[] }>('/audit', {
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
