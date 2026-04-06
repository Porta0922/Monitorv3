import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { AppInfo, Device, USBEvent, WifiEvent } from '../types';

type TabKey = 'activity' | 'inventory' | 'usb' | 'wifi';
type WifiStateFilter = 'all' | 'connected' | 'disconnected';
type WifiDateFilter = 'all' | 'today' | 'selected';

interface HistoryItem {
  app: string;
  title: string;
  seconds: number;
  duration: string;
  intervals: number;
  is_idle: boolean;
}

interface HourlyItem {
  hour: number;
  label: string;
  active_seconds: number;
  idle_seconds: number;
}

interface WifiEventWithDuration extends WifiEvent {
  connected_duration_seconds: number;
}

export function DeviceDetailPage() {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();

  const [device, setDevice] = useState<Device | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [hourly, setHourly] = useState<HourlyItem[]>([]);

  const [inventory, setInventory] = useState<AppInfo[]>([]);
  const [usbEvents, setUsbEvents] = useState<USBEvent[]>([]);
  const [wifiEvents, setWifiEvents] = useState<WifiEvent[]>([]);
  const [tab, setTab] = useState<TabKey>('activity');
  const [wifiStateFilter, setWifiStateFilter] = useState<WifiStateFilter>('all');
  const [wifiDateFilter, setWifiDateFilter] = useState<WifiDateFilter>('today');
  const [isLoading, setIsLoading] = useState(true);
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState('');

  const dateToday = useMemo(() => new Date().toISOString().slice(0, 10), []);
  const [selectedDate, setSelectedDate] = useState<string>(() => new Date().toISOString().slice(0, 10));

  useEffect(() => {
    if (!deviceId) return;

    const load = async () => {
      setIsLoading(true);
      setError('');

      try {
        const [devices, apps, usb, wifi] = await Promise.all([
          apiClient.getDevices(),
          apiClient.getApps(deviceId).catch(() => []),
          apiClient.getUsbHistory(deviceId, 250).catch(() => []),
          apiClient.getWifiHistory(deviceId, 250).catch(() => []),
        ]);

        const selected = devices.find((d) => d.device_id === deviceId) || null;
        setDevice(selected);
        setInventory(apps);
        setUsbEvents(usb);
        setWifiEvents(wifi);

        const [historyData, hourlyData] = await Promise.all([
          apiClient.getHistory(deviceId, selectedDate).catch(() => []),
          apiClient.getHourly(deviceId, selectedDate).catch(() => []),
        ]);

        setHistory(historyData as HistoryItem[]);
        setHourly(hourlyData as HourlyItem[]);
      } catch (err: any) {
        setError(err?.message || 'No fue posible cargar la consola de dispositivo.');
      } finally {
        setIsLoading(false);
      }
    };

    load();
  }, [deviceId, selectedDate]);

  const tabStats = useMemo(
    () => ({
      activity: history.length,
      inventory: inventory.length,
      usb: usbEvents.length,
      wifi: wifiEvents.length,
    }),
    [history.length, inventory.length, usbEvents.length, wifiEvents.length]
  );

  const formatDuration = (seconds?: number) => {
    const safeSeconds = Math.max(0, seconds || 0);
    const minutes = Math.floor(safeSeconds / 60);
    const remSeconds = safeSeconds % 60;
    return `${minutes}m ${remSeconds}s`;
  };

  const shortAppName = (rawName: string) => {
    const normalized = rawName.replace(/\\/g, '/');
    const lastSegment = normalized.split('/').pop() || rawName;
    return lastSegment.length > 38 ? `${lastSegment.slice(0, 35)}...` : lastSegment;
  };

  const tabClass = (key: TabKey) =>
    `rounded-full border px-4 py-2 text-xs font-semibold tracking-wide transition-all ${
      tab === key
        ? 'border-[#00d9ff] bg-[#00d9ff]/15 text-[#00d9ff]'
        : 'border-[#223462] bg-[#111a35] text-[#8ea0cf] hover:border-[#00d9ff]/50 hover:text-[#dce6ff]'
    }`;

  const maxHourlyValue = Math.max(1, ...hourly.map((item) => item.active_seconds + item.idle_seconds));

  const wifiEventsWithDuration = useMemo<WifiEventWithDuration[]>(() => {
    return wifiEvents.map((event, index) => {
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
    return wifiEventsWithDuration.filter((event) => {
      const stateMatches =
        wifiStateFilter === 'all' ? true : event.state.toLowerCase() === wifiStateFilter;

      const eventDate = new Date(event.timestamp).toISOString().slice(0, 10);
      const dateMatches =
        wifiDateFilter === 'all'
          ? true
          : eventDate === dateToday;

      return stateMatches && dateMatches;
    });
  }, [wifiEventsWithDuration, wifiStateFilter, wifiDateFilter, dateToday]);

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
      title={device ? `Consola de Dispositivo: ${device.nickname || device.hostname}` : 'Consola de Dispositivo'}
      subtitle={deviceId || 'Sin dispositivo'}
      actions={
        <button
          onClick={() => navigate('/dashboard')}
          className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-4 py-2 text-xs font-semibold tracking-wide text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Volver a AME
        </button>
      }
    >
      {device && (
        <section className="rounded-2xl border border-[#1b2b56] bg-[linear-gradient(160deg,#0f1d43,#0b1329)] p-5 shadow-[0_12px_26px_rgba(0,0,0,0.32)]">
          <div className="grid grid-cols-9 gap-4 text-sm">
            <div className="col-span-2">
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Hostname</p>
              <p className="mt-1 font-display text-base text-[#e4e6eb]">{device.hostname}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Estado</p>
              <p className={`mt-1 font-mono text-sm ${device.online ? 'text-[#00ff88]' : 'text-red-400'}`}>
                {device.online ? 'ONLINE' : 'OFFLINE'}
              </p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Activo hoy</p>
              <p className="mt-1 font-mono text-sm text-[#00d9ff]">{formatDuration(device.active_time_today_seconds)}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Inactivo hoy</p>
              <p className="mt-1 font-mono text-sm text-[#ff9f1a]">{formatDuration(device.idle_time_today_seconds)}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Teclas hoy</p>
              <p className="mt-1 font-mono text-sm text-[#00ff88]">{(device.keys_today || 0).toLocaleString()}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Mouse mov hoy</p>
              <p className="mt-1 font-mono text-sm text-[#00d9ff]">{(device.mouse_moves_today || 0).toLocaleString()}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Clicks hoy</p>
              <p className="mt-1 font-mono text-sm text-[#ffd54a]">{(device.mouse_clicks_today || 0).toLocaleString()}</p>
            </div>
            <div>
              <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">MAC</p>
              <p className="mt-1 font-mono text-sm text-[#a7b5dc]">{device.mac_address || 'N/A'}</p>
            </div>
          </div>
        </section>
      )}

      <section className="flex items-center justify-between gap-3">
        <div className="flex gap-2">
          <button className={tabClass('activity')} onClick={() => setTab('activity')}>
            Actividad ({tabStats.activity})
          </button>
          <button className={tabClass('inventory')} onClick={() => setTab('inventory')}>
            Inventario ({tabStats.inventory})
          </button>
          <button className={tabClass('usb')} onClick={() => setTab('usb')}>
            USB ({tabStats.usb})
          </button>
          <button className={tabClass('wifi')} onClick={() => setTab('wifi')}>
            WiFi ({tabStats.wifi})
          </button>
        </div>

        {tab === 'activity' && (
          <div className="flex items-center gap-2">
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Fecha</span>
            <input
              type="date"
              value={selectedDate}
              max={dateToday}
              onChange={(e) => setSelectedDate(e.target.value || dateToday)}
              className="rounded-full border border-[#223462] bg-[#111a35] px-3 py-1.5 font-mono text-[11px] text-[#dce6ff] cursor-pointer"
            />
            <button
              onClick={handleExportCsv}
              disabled={isExporting}
              className="rounded-full border border-[#00ff88]/40 bg-[#00ff88]/10 px-3 py-1.5 font-mono text-[11px] text-[#00ff88] hover:border-[#00ff88] disabled:opacity-60"
            >
              {isExporting ? 'Exportando...' : 'Export CSV'}
            </button>
          </div>
        )}

        {tab === 'wifi' && (
          <div className="flex items-center gap-2">
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Estado</span>
            <select
              value={wifiStateFilter}
              onChange={(event) => setWifiStateFilter(event.target.value as WifiStateFilter)}
              className="rounded-full border border-[#223462] bg-[#111a35] px-3 py-1.5 text-[11px] text-[#dce6ff]"
            >
              <option value="all">Todos</option>
              <option value="connected">Conectado</option>
              <option value="disconnected">Desconectado</option>
            </select>

            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Fecha</span>
            <select
              value={wifiDateFilter}
              onChange={(event) => setWifiDateFilter(event.target.value as WifiDateFilter)}
              className="rounded-full border border-[#223462] bg-[#111a35] px-3 py-1.5 text-[11px] text-[#dce6ff]"
            >
              <option value="today">Hoy</option>
              <option value="selected">Fecha seleccionada</option>
              <option value="all">Todo</option>
            </select>
          </div>
        )}
      </section>

      {tab === 'activity' && !isLoading && (
        <section className="rounded-2xl border border-[#1b2b56] bg-[linear-gradient(160deg,#0f1d43,#0b1329)] p-4 shadow-[0_12px_26px_rgba(0,0,0,0.32)]">
          <p className="mb-3 font-mono text-[11px] uppercase tracking-[0.18em] text-[#8ea0cf]">Actividad por Hora</p>
          <div className="flex h-32 items-end gap-1 overflow-hidden rounded-xl border border-[#20315a] bg-[#0a122a] p-2">
            {hourly.map((item) => {
              const total = item.active_seconds + item.idle_seconds;
              const activePct = total > 0 ? Math.round((item.active_seconds / maxHourlyValue) * 100) : 0;
              const idlePct = total > 0 ? Math.round((item.idle_seconds / maxHourlyValue) * 100) : 0;

              return (
                <div key={item.hour} className="flex min-w-[22px] flex-1 flex-col items-center justify-end gap-1">
                  <div className="relative flex h-20 w-3 flex-col justify-end overflow-hidden rounded-full bg-[#1b2a4f]">
                    <div className="w-full bg-[#ff9f1a]" style={{ height: `${idlePct}%` }} />
                    <div className="w-full bg-[#00d9ff]" style={{ height: `${activePct}%` }} />
                  </div>
                  <span className="font-mono text-[9px] text-[#7c90c1]">{item.hour}</span>
                </div>
              );
            })}
          </div>
        </section>
      )}

      {error && <section className="rounded-xl border border-red-500/30 bg-red-500/10 px-5 py-4 text-sm text-red-300">{error}</section>}

      {isLoading ? (
        <section className="rounded-2xl border border-[#1b2b56] bg-[#0b1329] px-6 py-10 text-center text-[#a0a5b2]">Cargando telemetria...</section>
      ) : (
        <section className="overflow-hidden rounded-2xl border border-[#1b2b56] bg-[linear-gradient(160deg,#0f1d43,#0b1329)] shadow-[0_12px_26px_rgba(0,0,0,0.32)]">
          {tab === 'activity' && (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr>
                    <th>Aplicacion</th>
                    <th>Ventana</th>
                    <th>Duracion</th>
                    <th>Intervalos</th>
                    <th>Estado</th>
                  </tr>
                </thead>
                <tbody>
                  {history.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="py-8 text-center text-[#8fa0c9]">No hay actividad para esta fecha.</td>
                    </tr>
                  ) : (
                    history.map((row, idx) => (
                      <tr key={`${row.app}-${idx}`}>
                        <td className="max-w-[320px] truncate font-mono text-[12px] text-[#dce6ff]" title={row.app}>
                          {shortAppName(row.app)}
                        </td>
                        <td className="max-w-[540px] truncate text-[12px] text-[#9eb0dc]" title={row.title}>
                          {row.title || 'Sin titulo'}
                        </td>
                        <td className="whitespace-nowrap font-mono text-[11px] text-[#00ff88]">{row.duration}</td>
                        <td className="whitespace-nowrap font-mono text-[11px] text-[#9eb0dc]">{row.intervals}</td>
                        <td>
                          <span className={`inline-flex rounded-full px-2.5 py-1 font-mono text-[10px] ${row.is_idle ? 'border border-[#ff9f1a]/40 bg-[#ff9f1a]/10 text-[#ff9f1a]' : 'border border-[#00d9ff]/40 bg-[#00d9ff]/10 text-[#00d9ff]'}`}>
                            {row.is_idle ? 'IDLE' : 'ACTIVE'}
                          </span>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          )}

          {tab === 'inventory' && (
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
                      <td colSpan={4} className="py-8 text-center text-[#8fa0c9]">Sin datos de inventario para este equipo.</td>
                    </tr>
                  ) : (
                    inventory.map((app, idx) => (
                      <tr key={`${app.app_name}-${idx}`}>
                        <td className="max-w-[360px] truncate text-[12px] text-[#dce6ff]" title={app.app_name}>{app.app_name}</td>
                        <td className="text-[12px] text-[#9eb0dc]">{app.version || 'Unknown'}</td>
                        <td>
                          <span className={`inline-flex rounded-full px-2.5 py-1 font-mono text-[10px] ${app.verified ? 'border border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]' : 'border border-red-500/40 bg-red-500/10 text-red-300'}`}>
                            {app.verified ? 'VERIFIED' : 'UNVERIFIED'}
                          </span>
                        </td>
                        <td className="max-w-[440px] truncate font-mono text-[11px] text-[#91a3d2]" title={app.exe_hash}>{app.exe_hash}</td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          )}

          {tab === 'usb' && (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr>
                    <th>Hora</th>
                    <th>Dispositivo</th>
                    <th>Serial</th>
                    <th>Accion</th>
                  </tr>
                </thead>
                <tbody>
                  {usbEvents.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="py-8 text-center text-[#8fa0c9]">Sin eventos USB para este equipo.</td>
                    </tr>
                  ) : (
                    usbEvents.map((event, idx) => (
                      <tr key={`${event.timestamp}-${idx}`}>
                        <td className="whitespace-nowrap font-mono text-[11px] text-[#9eb0dc]">{new Date(event.timestamp).toLocaleString()}</td>
                        <td className="max-w-[380px] truncate text-[12px] text-[#dce6ff]" title={event.device_name}>{event.device_name}</td>
                        <td className="max-w-[340px] truncate font-mono text-[11px] text-[#91a3d2]" title={event.serial_number}>{event.serial_number || 'N/A'}</td>
                        <td>
                          <span className={`inline-flex rounded-full px-2.5 py-1 font-mono text-[10px] ${event.action === 'IN' ? 'border border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]' : 'border border-red-500/40 bg-red-500/10 text-red-300'}`}>
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
              <div className="grid grid-cols-5 gap-3 rounded-xl border border-[#20315a] bg-[#0a122a] px-4 py-3">
                <div className="col-span-2">
                  <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Red actual</p>
                  <p className="mt-1 truncate font-display text-base text-[#e4e6eb]" title={currentWifiEvent?.ssid || 'Sin conexion'}>
                    {currentWifiEvent?.ssid || 'Sin conexion'}
                  </p>
                </div>
                <div>
                  <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Estado</p>
                  <p className={`mt-1 font-mono text-sm ${currentWifiEvent?.state === 'connected' ? 'text-[#00ff88]' : 'text-[#ff9f1a]'}`}>
                    {(currentWifiEvent?.state || 'unknown').toUpperCase()}
                  </p>
                </div>
                <div>
                  <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Senal</p>
                  <p className="mt-1 font-mono text-sm text-[#00d9ff]">{currentWifiEvent?.signal_percent !== undefined ? `${currentWifiEvent.signal_percent}%` : 'N/A'}</p>
                </div>
                <div>
                  <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Ultimo cambio</p>
                  <p className="mt-1 truncate font-mono text-[11px] text-[#a7b5dc]" title={currentWifiEvent?.timestamp}>
                    {currentWifiEvent ? new Date(currentWifiEvent.timestamp).toLocaleString() : 'N/A'}
                  </p>
                </div>
              </div>

              <div className="rounded-xl border border-[#20315a] bg-[#0a122a] px-4 py-3">
                <div className="mb-2 flex items-center justify-between">
                  <p className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Top redes por tiempo conectado</p>
                  <span className="font-mono text-[10px] text-[#8ea0cf]">{wifiTotalsBySsid.length} SSID</span>
                </div>

                {wifiTotalsBySsid.length === 0 ? (
                  <p className="font-mono text-[11px] text-[#8fa0c9]">Sin tiempo conectado acumulado para el filtro actual.</p>
                ) : (
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                    {wifiTotalsBySsid.map((item) => (
                      <div key={item.ssid} className="flex items-center justify-between rounded-lg border border-[#243665] bg-[#0d1733] px-3 py-2">
                        <p className="max-w-[70%] truncate font-mono text-[11px] text-[#dce6ff]" title={item.ssid}>{item.ssid}</p>
                        <p className="font-mono text-[11px] text-[#00ff88]">{formatLongDuration(item.seconds)}</p>
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
                        <td colSpan={7} className="py-8 text-center text-[#8fa0c9]">Sin historial WiFi para el filtro seleccionado.</td>
                      </tr>
                    ) : (
                      filteredWifiEvents.map((event, idx) => (
                        <tr key={`${event.timestamp}-${idx}`}>
                          <td className="whitespace-nowrap font-mono text-[11px] text-[#9eb0dc]">{new Date(event.timestamp).toLocaleString()}</td>
                          <td className="max-w-[280px] truncate text-[12px] text-[#dce6ff]" title={event.ssid}>{event.ssid || 'N/A'}</td>
                          <td className="max-w-[280px] truncate font-mono text-[11px] text-[#91a3d2]" title={event.bssid}>{event.bssid || 'N/A'}</td>
                          <td className="whitespace-nowrap font-mono text-[11px] text-[#00d9ff]">{event.signal_percent !== undefined ? `${event.signal_percent}%` : 'N/A'}</td>
                          <td className="whitespace-nowrap font-mono text-[11px] text-[#00ff88]">{event.state === 'connected' ? formatLongDuration(event.connected_duration_seconds) : '--'}</td>
                          <td>
                            <span className={`inline-flex rounded-full px-2.5 py-1 font-mono text-[10px] ${event.state === 'connected' ? 'border border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]' : 'border border-[#ff9f1a]/40 bg-[#ff9f1a]/10 text-[#ff9f1a]'}`}>
                              {event.state.toUpperCase()}
                            </span>
                          </td>
                          <td className="max-w-[220px] truncate font-mono text-[11px] text-[#91a3d2]" title={event.interface_name}>{event.interface_name}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </section>
      )}
    </AppShell>
  );
}
