import {SessionActivityPanel} from '../components/metrics/SessionActivityPanel';
import {AgentCredentialsPanel} from '../components/AgentCredentialsPanel';
import {DeviceOverview} from '../components/DeviceOverview';
import {DeviceActivityExplorer} from '../components/DeviceActivityExplorer';
import type {HourGroup} from '../api/desktop';
import React, { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import { InventoryStatePanel } from '../components/InventoryStatePanel';
import { DeviceTelemetryPanel } from '../components/metrics/DeviceTelemetryPanel';
import { DailyMetricsPanel } from '../components/metrics/DailyMetricsPanel';
import type { AppInfo, Device, RunningAppInfo, USBEvent, WifiEvent, NodeResourceMetric } from '../types';

type TabKey = 'overview' | 'activity' | 'inventory' | 'usb' | 'wifi' | 'resources' | 'diagnostic';
type WifiStateFilter = 'all' | 'connected' | 'disconnected';

interface HourlyProgramItem {
  app: string;
  seconds: number;
  duration: string;
  intervals: number;
  is_idle: boolean;
  window_count?: number;
  windows?: Array<{
    title: string;
    seconds: number;
    duration: string;
    intervals: number;
  }>;
}

interface HourlyProgramsGroup {
  hour: number;
  label: string;
  programs: HourlyProgramItem[];
}

interface UsbSummaryRow {
  key: string;
  device_key: string;
  timestamp: string;
  device_name: string;
  serial_number?: string;
  hardware_id: string;
  action: 'IN' | 'OUT';
  pair_id: string;
  pair_step: number;
  pair_started_at: number;
}

interface WifiEventWithDuration extends WifiEvent {
  connected_duration_seconds: number;
}

export function DeviceDetailPage() {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();

  const [device, setDevice] = useState<Device | null>(null);
  const [hourlyPrograms, setHourlyPrograms] = useState<HourlyProgramsGroup[]>([]);
  const [resourceMetrics, setResourceMetrics] = useState<NodeResourceMetric[]>([]);

  const [inventory, setInventory] = useState<AppInfo[]>([]);
  const [runningApps, setRunningApps] = useState<RunningAppInfo[]>([]);
  const [usbEvents, setUsbEvents] = useState<USBEvent[]>([]);
  const [wifiEvents, setWifiEvents] = useState<WifiEvent[]>([]);
  const [tab, setTab] = useState<TabKey>('overview');
  const [wifiStateFilter, setWifiStateFilter] = useState<WifiStateFilter>('all');
  const [isLoading, setIsLoading] = useState(true);
  const [isExporting, setIsExporting] = useState(false);
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState('');

  const dateToday = useMemo(() => new Date().toISOString().slice(0,10), []);
  const [selectedDate, setSelectedDate] = useState<string>(() => new Date().toISOString().slice(0,10));

  useEffect(() => {
    if (!deviceId) return;

    let alive = true;
    const load = async () => {
      setIsLoading(true);
      setDevice(null);
      setHourlyPrograms([]);
      setResourceMetrics([]); setInventory([]); setRunningApps([]); setUsbEvents([]); setWifiEvents([]);
      setError('');

      try {
        const [devices, apps, openApps, usb, wifi, resources] = await Promise.all([
          apiClient.getDevices(),
          apiClient.getApps(deviceId),
          apiClient.getRunningApps(deviceId),
          apiClient.getUsbHistory(deviceId, 250, selectedDate),
          apiClient.getWifiHistory(deviceId, 250, selectedDate),
          apiClient.getDeviceResources(deviceId, selectedDate, 2880),
        ]);

        if (!alive) return;
        const selected = devices.find((d) => d.device_id === deviceId) || null;
        if (!selected) throw new Error('El equipo no existe o ya no está disponible.');
        setDevice(selected);
        setInventory(apps);
        setRunningApps(openApps);
        setUsbEvents(usb);
        setWifiEvents(wifi);
        setResourceMetrics(resources);

        const hourlyProgramsData = await apiClient.getHistoryHourlyPrograms(deviceId, selectedDate);

        if (!alive) return;
        setHourlyPrograms(hourlyProgramsData as HourlyProgramsGroup[]);
      } catch (err: unknown) {
        if (alive) setError(err instanceof Error ? err.message : 'No se pudo consultar el detalle. Vuelve a seleccionar el día para reintentar.');
      } finally {
        if (alive) setIsLoading(false);
      }
    };

    void load();
    return () => { alive = false; };
  }, [deviceId, selectedDate, revision]);

  const formatTopProcessCpu = (value?: number) => {
    return Math.round(Math.min(100, Math.max(0, value || 0)));
  };

  const formatTopProcessMemoryMb = (value?: number) => {
    const raw = Math.max(0, value || 0);
    // Backward compatibility for legacy rows persisted as KB-as-MB.
    const normalized = raw > 8192 ? raw / 1024 : raw;
    return Math.round(normalized);
  };

  const shortAppName = (rawName: string) => {
    const cleaned = (rawName || '').trim();
    const normalized = cleaned.toLowerCase();
    if (!cleaned || normalized === 'unknown' || normalized === 'n/a' || normalized === '<unknown>' || normalized === '(unknown)') {
      return 'Sin identificar';
    }

    const normalizedPath = cleaned.replace(/\\/g, '/');
    const lastSegment = normalizedPath.split('/').pop() || cleaned;
    const aliasMap: Record<string, string> = {
      'chrome.exe': 'Google Chrome',
      'msedge.exe': 'Microsoft Edge',
      'firefox.exe': 'Mozilla Firefox',
      'code.exe': 'Visual Studio Code',
    };
    const mapped = aliasMap[lastSegment.toLowerCase()];
    if (mapped) {
      return mapped;
    }
    return lastSegment.length > 38 ? `${lastSegment.slice(0, 35)}...` : lastSegment;
  };

  const tabClass = (key: TabKey) =>
    `rounded-full border px-4 py-2 text-xs font-semibold tracking-wide transition-all ${
      tab === key
        ? 'border-[var(--palette-dfe7e3)] bg-[var(--palette-17715a)]/15 text-[var(--palette-17715a)]'
        : 'border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] text-[var(--palette-53685c)] hover:border-[var(--palette-dfe7e3)]/50 hover:text-[var(--palette-53685c)]'
    }`;

  const resourcePeak = useMemo(() => {
    if (resourceMetrics.length === 0) {
      return null;
    }

    return resourceMetrics.reduce((best: NodeResourceMetric | null, current: NodeResourceMetric) => {
      if (!best) return current;
      const bestScore = (best.cpu_percent || 0) + (best.memory_percent || 0);
      const currentScore = (current.cpu_percent || 0) + (current.memory_percent || 0);
      return currentScore > bestScore ? current : best;
    }, resourceMetrics[0] || null);
  }, [resourceMetrics]);

  const usbSummaryRows = useMemo<UsbSummaryRow[]>(() => {
    const normalized = (value?: string) => (value || '').trim();
    const isUnknownLike = (value?: string) => {
      const v = normalized(value).toLowerCase();
      return !v || v === 'unknown' || v === 'n/a' || v === '<unknown>' || v === '(unknown)';
    };

    const eventsByDevice = new Map<string, USBEvent[]>();

    for (const event of usbEvents) {
      const serial = isUnknownLike(event.serial_number) ? '' : normalized(event.serial_number);
      const hardware = isUnknownLike(event.hardware_id) ? '' : normalized(event.hardware_id);
      const name = normalized(event.device_name);

      const uniqueKey = serial || hardware || name;

      if (!uniqueKey) {
        continue;
      }

      const list = eventsByDevice.get(uniqueKey) || [];
      list.push(event);
      eventsByDevice.set(uniqueKey, list);
    }

    const pairedRows: UsbSummaryRow[] = [];

    for (const [deviceKey, events] of eventsByDevice.entries()) {
      const sortedAsc = [...events].sort(
        (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
      );

      let pairCounter = 0;
      let hasOpenConnection = false;
      let pairStartedAt = 0;

      for (const event of sortedAsc) {
        if (event.action === 'IN' || !hasOpenConnection) {
          pairCounter += 1;
          pairStartedAt = new Date(event.timestamp).getTime();
        }

        const pairId = `${deviceKey}#${pairCounter}`;

        pairedRows.push({
          key: `${pairId}|${event.action}|${event.timestamp}`,
          device_key: deviceKey,
          timestamp: event.timestamp,
          device_name: event.device_name,
          serial_number: event.serial_number,
          hardware_id: event.hardware_id,
          action: event.action,
          pair_id: pairId,
          pair_step: event.action === 'IN' ? 1 : 2,
          pair_started_at: pairStartedAt,
        });

        if (event.action === 'IN') {
          hasOpenConnection = true;
        }
        if (event.action === 'OUT') {
          hasOpenConnection = false;
        }
      }
    }

    return pairedRows.sort((a, b) => {
      const pairCompare = b.pair_started_at - a.pair_started_at;
      if (pairCompare !== 0) return pairCompare;
      const stepCompare = a.pair_step - b.pair_step;
      if (stepCompare !== 0) return stepCompare;
      return new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
    });
  }, [usbEvents]);

  const wifiEventsWithDuration = useMemo<WifiEventWithDuration[]>(() => {
    return wifiEvents.map((event: WifiEvent, index: number) => {
      const startedAtMs = new Date(event.timestamp).getTime();
      const endedAtMs =
        index === 0
          ? Date.now()
          : new Date(wifiEvents[index - 1].timestamp).getTime();

      const connectedDurationSeconds =
        event.state === 'connected'
          ? Math.max(0, Math.floor((endedAtMs - startedAtMs) / 1000))
          : 0;

      return {
        ...event,
        connected_duration_seconds: connectedDurationSeconds,
      };
    });
  }, [wifiEvents]);

  const filteredWifiEvents = useMemo(() => {
    return wifiEventsWithDuration.filter((event: WifiEventWithDuration) => {
      const stateMatches =
        wifiStateFilter === 'all' ? true : event.state.toLowerCase() === wifiStateFilter;

      const eventDate = new Date(event.timestamp).toISOString().slice(0, 10);
      const dateMatches = eventDate === selectedDate;

      return stateMatches && dateMatches;
    });
  }, [wifiEventsWithDuration, wifiStateFilter, selectedDate]);

  const currentWifiEvent = useMemo(() => {
    return wifiEventsWithDuration[0] || null;
  }, [wifiEventsWithDuration]);

  const wifiTotalsBySsid = useMemo(() => {
    const totals = new Map<string, number>();

    for (const event of filteredWifiEvents) {
      if (event.state !== 'connected' || event.connected_duration_seconds <= 0) {
        continue;
      }

      const ssidKey = event.ssid?.trim() ? event.ssid.trim() : '(sin SSID)';
      totals.set(ssidKey, (totals.get(ssidKey) || 0) + event.connected_duration_seconds);
    }

    return Array.from(totals.entries())
      .map(([ssid, seconds]) => ({ ssid, seconds }))
      .sort((a, b) => b.seconds - a.seconds)
      .slice(0, 6);
  }, [filteredWifiEvents]);

  const formatLongDuration = (seconds?: number) => {
    const safeSeconds = Math.max(0, seconds || 0);
    const hours = Math.floor(safeSeconds / 3600);
    const minutes = Math.floor((safeSeconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  const handleExportCsv = async () => {
    if (!deviceId) return;
    setIsExporting(true);
    try {
      const blob = await apiClient.exportCsv({
        deviceId,
        from: selectedDate,
        to: selectedDate,
      });

      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ame_${deviceId.slice(0, 8)}_${selectedDate}.csv`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(url);
    } catch {
      setError('No fue posible exportar CSV.');
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <AppShell
      currentPage="dashboard"
      contentClassName="device-detail"
      title={device ? device.nickname || device.hostname : 'Detalle del equipo'}
      subtitle="Recursos, actividad y conexiones del equipo seleccionado."
      actions={
        <button
          onClick={() => navigate('/dashboard')}
          className="rounded-full border border-[var(--palette-dfe7e3)]/50 bg-[var(--palette-17715a)]/10 px-4 py-2 text-xs font-semibold tracking-wide text-[var(--palette-17715a)] hover:border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-17715a)]/20"
        >
          Volver a los equipos
        </button>
      }
    >
      {device && <div className="device-identity"><span className={device.online?'device-online':'device-offline'}>{device.online?'Conectado':'Sin conexión'}</span><strong>{device.hostname}</strong><span className="muted">Última señal: {new Date(device.last_seen).toLocaleString()}</span><details><summary>Identificación</summary><p>Identificador: {device.device_id}</p><p>MAC: {device.mac_address||'Sin informar'}</p></details><button className="button button-small" disabled={isLoading} onClick={()=>setRevision(v=>v+1)}>{isLoading?'Actualizando…':'Actualizar'}</button></div>}

      <section className="device-navigation">
        <nav className="device-tabs" aria-label="Secciones del equipo">{([['overview','Resumen'],['activity','Actividad'],['inventory','Aplicaciones'],['resources','Recursos'],['usb','USB'],['wifi','WiFi'],['diagnostic','Diagnóstico']] as [TabKey,string][]).map(([key,label])=><button key={key} className={tabClass(key)} aria-current={tab===key?'page':undefined} onClick={()=>setTab(key)}>{label}</button>)}</nav>

        {(tab === 'overview' || tab === 'activity' || tab === 'resources' || tab === 'usb' || tab === 'wifi') && (
          <div className="flex items-center gap-2">
            <label htmlFor="device-date" className="muted">Fecha de consulta (UTC)</label>
            <input
              type="date" id="device-date"
              value={selectedDate}
              max={dateToday}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSelectedDate(e.target.value || dateToday)}
              className="rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-1.5 font-sans text-[12px] text-[var(--palette-53685c)] cursor-pointer"
            />
            <button
              onClick={handleExportCsv}
              disabled={isExporting}
              className="rounded-full border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 px-3 py-1.5 font-sans text-[12px] text-[var(--palette-31764d)] hover:border-[var(--palette-dfe7e3)] disabled:opacity-60"
            >
              {isExporting ? 'Exportando…' : 'Exportar actividad'}
            </button>
          </div>
        )}

        {tab === 'wifi' && (
          <div className="flex items-center gap-2">
            <span className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Estado</span>
            <select
              value={wifiStateFilter}
              onChange={(event: React.ChangeEvent<HTMLSelectElement>) => setWifiStateFilter(event.target.value as WifiStateFilter)}
              className="rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-1.5 text-[12px] text-[var(--palette-53685c)]"
            >
              <option value="all">Todos</option>
              <option value="connected">Conectado</option>
              <option value="disconnected">Desconectado</option>
            </select>
          </div>
        )}
      </section>

      {tab === 'resources' && !error && !isLoading && resourceMetrics.length > 0 && (
        <section className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-4 py-2.5">
          <span className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Recursos: {selectedDate}</span>
          <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)]/30 bg-[var(--palette-b2433d)]/10 px-2.5 py-1">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--palette-b2433d)]" />
            <span className="font-sans text-[12px] text-[var(--palette-53685c)]">CPU pico {Math.min(100, Math.round(Math.max(...resourceMetrics.map(metric => metric.cpu_percent))))}%</span>
          </span>
          <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)]/30 bg-[var(--palette-17715a)]/10 px-2.5 py-1">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--palette-17715a)]" />
            <span className="font-sans text-[12px] text-[var(--palette-53685c)]">RAM pico {Math.min(100, Math.round(Math.max(...resourceMetrics.map(metric => metric.memory_percent))))}%</span>
          </span>
          {resourcePeak?.top_process_name && (
            <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-2.5 py-1">
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">▸</span>
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">{resourcePeak.top_process_name}</span>
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">
                {formatTopProcessCpu(resourcePeak.top_process_cpu_percent)}% cpu · {formatTopProcessMemoryMb(resourcePeak.top_process_memory_mb)} MB
              </span>
            </span>
          )}
          <span className="ml-auto font-sans text-[12px] text-[var(--palette-53685c)]">{resourceMetrics.length} muestras</span>
        </section>
      )}

      {deviceId && tab === 'inventory' && <InventoryStatePanel key={deviceId} deviceId={deviceId} />}
      {deviceId && tab === 'diagnostic' && <><DeviceTelemetryPanel key={deviceId} deviceId={deviceId} />{apiClient.getRole()==='admin' && <AgentCredentialsPanel key={`credentials-${deviceId}`} deviceId={deviceId}/>}</>}
      {deviceId && tab === 'overview' && !isLoading && !error && <DeviceOverview key={deviceId} deviceId={deviceId} groups={hourlyPrograms as HourGroup[]} onActivity={()=>setTab('activity')} onResources={()=>setTab('resources')} onDiagnostic={()=>setTab('diagnostic')}/>}
      {deviceId && tab === 'overview' && !isLoading && !error && <SessionActivityPanel key={`${deviceId}-${selectedDate}`} deviceId={deviceId} initialDay={selectedDate} compact/>}
      {deviceId && tab === 'resources' && <DeviceTelemetryPanel key={deviceId} deviceId={deviceId} resourcesOnly />}
      {tab === 'activity' && !isLoading && !error && <DeviceActivityExplorer key={`${deviceId}-${selectedDate}`} groups={hourlyPrograms as HourGroup[]}/>}


      {tab === 'resources' && !error && !isLoading && deviceId && (
        <DailyMetricsPanel deviceId={deviceId} endDate={selectedDate} />
      )}

      {error && <section className="notice notice-error" role="alert">{error} <button className="text-button" onClick={() => setRevision(v => v + 1)}>Reintentar</button></section>}

      {error ? null : isLoading ? (
        <section className="rounded-2xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-6 py-10 text-center text-[var(--palette-53685c)]">Consultando datos del equipo…</section>
      ) : ['inventory','usb','wifi'].includes(tab) ? (
        <section className="overflow-hidden rounded-2xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] shadow-sm">
          {tab === 'inventory' && (
            <div className="space-y-6">
              <section className="space-y-3">
                <div>
                  <h3 className="text-sm font-semibold text-[var(--palette-53685c)]">Últimas ventanas reportadas</h3>
                  <p className="text-xs text-[var(--palette-53685c)]">Reportes por sesión de escritorio. Incluso un reporte reciente no confirma que la ventana siga abierta. Los de más de tres minutos se marcan como antiguos.</p>
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr>
                        <th>Aplicacion</th>
                        <th>Ventanas</th>
                        <th>Ventana principal</th>
                        <th>Sesión y reporte</th>
                        <th>Ruta</th>
                      </tr>
                    </thead>
                    <tbody>
                      {runningApps.length === 0 ? (
                        <tr>
                          <td colSpan={5} className="py-8 text-center text-[var(--palette-53685c)]">No hay ventanas reportadas. Esto no permite distinguir entre ausencia de ventanas y falta de captura.</td>
                        </tr>
                      ) : (
                        runningApps.map((app: RunningAppInfo) => (
                          <tr key={app.id}>
                            <td className="max-w-[260px] truncate text-[12px] text-[var(--palette-53685c)]" title={app.app_name}>{shortAppName(app.app_name)}</td>
                            <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-17715a)]">{app.window_count}</td>
                            <td className="max-w-[420px] truncate text-[12px] text-[var(--palette-53685c)]" title={app.primary_title || 'Sin titulo'}>{app.primary_title || 'Sin titulo'}</td>
                            <td className="text-xs"><div>{app.stale === false ? 'Reporte reciente' : app.stale === true ? 'Reporte antiguo' : 'Vigencia sin confirmar'}</div><time dateTime={app.updated_at}>{new Date(app.updated_at).toLocaleString()}</time><div title={app.principal || undefined}>{app.principal || 'Sin atribución de usuario'}</div><div title={app.session_id || undefined}>{app.session_id ? `Sesión ${app.session_id.slice(0, 8)}` : 'Origen heredado'}</div></td>
                            <td className="max-w-[360px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={app.exe_path || ''}>{app.exe_path || 'No disponible'}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </section>

              <section className="space-y-3">
                <div>
                  <h3 className="text-sm font-semibold text-[var(--palette-53685c)]">Detecciones heredadas (no confirman instalación actual)</h3>
                  <p className="text-xs text-[var(--palette-53685c)]">Listado historico de software detectado en el equipo.</p>
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr>
                        <th>Aplicacion</th>
                        <th>Version</th>
                        <th>Estado</th>
                        <th>Hash</th>
                      </tr>
                    </thead>
                    <tbody>
                      {inventory.length === 0 ? (
                        <tr>
                          <td colSpan={4} className="py-8 text-center text-[var(--palette-53685c)]">Sin datos de inventario para este equipo.</td>
                        </tr>
                      ) : (
                        inventory.map((app: AppInfo, idx: number) => (
                          <tr key={`${app.app_name}-${idx}`}>
                            <td className="max-w-[360px] truncate text-[12px] text-[var(--palette-53685c)]" title={app.app_name}>{app.app_name}</td>
                            <td className="text-[12px] text-[var(--palette-53685c)]">{app.version || 'Unknown'}</td>
                            <td>
                              <span className={`inline-flex rounded-full px-2.5 py-1 font-sans text-[12px] ${app.verified ? 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 text-[var(--palette-31764d)]' : 'border border-red-500/40 bg-red-500/10 text-red-700'}`}>
                                {app.verified ? 'VERIFIED' : 'UNVERIFIED'}
                              </span>
                            </td>
                            <td className="max-w-[440px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={app.exe_hash}>{app.exe_hash}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </section>
            </div>
          )}

          {tab === 'usb' && (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr>
                    <th>Par</th>
                    <th>Ultimo evento</th>
                    <th>Dispositivo</th>
                    <th>Serial</th>
                    <th>Identificador</th>
                    <th>Accion</th>
                  </tr>
                </thead>
                <tbody>
                  {usbSummaryRows.length === 0 ? (
                    <tr>
                      <td colSpan={6} className="py-8 text-center text-[var(--palette-53685c)]">Sin eventos USB para este equipo.</td>
                    </tr>
                  ) : (
                    usbSummaryRows.map((event: UsbSummaryRow) => (
                      <tr key={event.key}>
                        <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-53685c)]">
                          {event.pair_id.split('#').pop()}
                        </td>
                        <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-53685c)]">{new Date(event.timestamp).toLocaleString()}</td>
                        <td className="max-w-[380px] truncate text-[12px] text-[var(--palette-53685c)]" title={event.device_name}>{event.device_name}</td>
                        <td className="max-w-[340px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={event.serial_number}>{event.serial_number || 'N/A'}</td>
                        <td className="max-w-[420px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={event.hardware_id}>{event.hardware_id || 'N/A'}</td>
                        <td>
                          <span className={`inline-flex rounded-full px-2.5 py-1 font-sans text-[12px] ${event.action === 'IN' ? 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 text-[var(--palette-31764d)]' : 'border border-red-500/40 bg-red-500/10 text-red-700'}`}>
                            {event.action === 'IN' ? 'CONNECTED' : 'DISCONNECTED'}
                          </span>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          )}

          {tab === 'wifi' && (
            <div className="space-y-3">
              <div className="grid grid-cols-5 gap-3 rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-4 py-3">
                <div className="col-span-2">
                  <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Red actual</p>
                  <p className="mt-1 truncate font-display text-base text-[var(--palette-53685c)]" title={currentWifiEvent?.ssid || 'Sin conexion'}>
                    {currentWifiEvent?.ssid || 'Sin conexion'}
                  </p>
                </div>
                <div>
                  <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Estado</p>
                  <p className={`mt-1 font-sans text-sm ${currentWifiEvent?.state === 'connected' ? 'text-[var(--palette-31764d)]' : 'text-[var(--palette-97621b)]'}`}>
                    {(currentWifiEvent?.state || 'unknown').toUpperCase()}
                  </p>
                </div>
                <div>
                  <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Senal</p>
                  <p className="mt-1 font-sans text-sm text-[var(--palette-17715a)]">{currentWifiEvent?.signal_percent !== undefined ? `${currentWifiEvent.signal_percent}%` : 'N/A'}</p>
                </div>
                <div>
                  <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Ultimo cambio</p>
                  <p className="mt-1 truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={currentWifiEvent?.timestamp}>
                    {currentWifiEvent ? new Date(currentWifiEvent.timestamp).toLocaleString() : 'N/A'}
                  </p>
                </div>
              </div>

              <div className="rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-4 py-3">
                <div className="mb-2 flex items-center justify-between">
                  <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Top redes por tiempo conectado</p>
                  <span className="font-sans text-[12px] text-[var(--palette-53685c)]">{wifiTotalsBySsid.length} SSID</span>
                </div>

                {wifiTotalsBySsid.length === 0 ? (
                  <p className="font-sans text-[12px] text-[var(--palette-53685c)]">Sin tiempo conectado acumulado para el filtro actual.</p>
                ) : (
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                    {wifiTotalsBySsid.map((item: { ssid: string; seconds: number }) => (
                      <div key={item.ssid} className="flex items-center justify-between rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2">
                        <p className="max-w-[70%] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={item.ssid}>{item.ssid}</p>
                        <p className="font-sans text-[12px] text-[var(--palette-31764d)]">{formatLongDuration(item.seconds)}</p>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr>
                      <th>Hora</th>
                      <th>SSID</th>
                      <th>BSSID</th>
                      <th>Senal</th>
                      <th>Duracion conectada</th>
                      <th>Estado</th>
                      <th>Interfaz</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredWifiEvents.length === 0 ? (
                      <tr>
                        <td colSpan={7} className="py-8 text-center text-[var(--palette-53685c)]">Sin historial WiFi para el filtro seleccionado.</td>
                      </tr>
                    ) : (
                      filteredWifiEvents.map((event: WifiEventWithDuration, idx: number) => (
                        <tr key={`${event.timestamp}-${idx}`}>
                          <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-53685c)]">{new Date(event.timestamp).toLocaleString()}</td>
                          <td className="max-w-[280px] truncate text-[12px] text-[var(--palette-53685c)]" title={event.ssid}>{event.ssid || 'N/A'}</td>
                          <td className="max-w-[280px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={event.bssid}>{event.bssid || 'N/A'}</td>
                          <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-17715a)]">{event.signal_percent !== undefined ? `${event.signal_percent}%` : 'N/A'}</td>
                          <td className="whitespace-nowrap font-sans text-[12px] text-[var(--palette-31764d)]">{event.state === 'connected' ? formatLongDuration(event.connected_duration_seconds) : '--'}</td>
                          <td>
                            <span className={`inline-flex rounded-full px-2.5 py-1 font-sans text-[12px] ${event.state === 'connected' ? 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 text-[var(--palette-31764d)]' : 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-97621b)]/10 text-[var(--palette-97621b)]'}`}>
                              {event.state.toUpperCase()}
                            </span>
                          </td>
                          <td className="max-w-[220px] truncate font-sans text-[12px] text-[var(--palette-53685c)]" title={event.interface_name}>{event.interface_name}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </section>
      ) : null}
    </AppShell>
  );
}
