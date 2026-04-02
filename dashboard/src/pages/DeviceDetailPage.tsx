import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { AppInfo, Device, USBEvent } from '../types';

type TabKey = 'activity' | 'inventory' | 'usb';

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

export function DeviceDetailPage() {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();

  const [device, setDevice] = useState<Device | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [hourly, setHourly] = useState<HourlyItem[]>([]);
  const [availableDates, setAvailableDates] = useState<string[]>([]);
  const [selectedDate, setSelectedDate] = useState('');

  const [inventory, setInventory] = useState<AppInfo[]>([]);
  const [usbEvents, setUsbEvents] = useState<USBEvent[]>([]);
  const [tab, setTab] = useState<TabKey>('activity');
  const [isLoading, setIsLoading] = useState(true);
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState('');

  const dateToday = useMemo(() => new Date().toISOString().slice(0, 10), []);

  useEffect(() => {
    if (!deviceId) return;

    const load = async () => {
      setIsLoading(true);
      setError('');

      try {
        const [devices, apps, usb, dates] = await Promise.all([
          apiClient.getDevices(),
          apiClient.getApps(deviceId).catch(() => []),
          apiClient.getUsbHistory(deviceId, 250).catch(() => []),
          apiClient.getAvailableDates(deviceId).catch(() => []),
        ]);

        const selected = devices.find((d) => d.device_id === deviceId) || null;
        setDevice(selected);
        setInventory(apps);
        setUsbEvents(usb);
        setAvailableDates(dates);

        const resolvedDate = selectedDate || dates[0] || dateToday;
        if (!selectedDate) {
          setSelectedDate(resolvedDate);
        }

        const [historyData, hourlyData] = await Promise.all([
          apiClient.getHistory(deviceId, resolvedDate).catch(() => []),
          apiClient.getHourly(deviceId, resolvedDate).catch(() => []),
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
  }, [deviceId, selectedDate, dateToday]);

  const tabStats = useMemo(
    () => ({
      activity: history.length,
      inventory: inventory.length,
      usb: usbEvents.length,
    }),
    [history.length, inventory.length, usbEvents.length]
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

  const handleExportCsv = async () => {
    if (!deviceId) return;
    setIsExporting(true);
    try {
      const blob = await apiClient.exportCsv({
        deviceId,
        from: selectedDate || dateToday,
        to: selectedDate || dateToday,
      });

      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ame_${deviceId.slice(0, 8)}_${selectedDate || dateToday}.csv`;
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
          <div className="grid grid-cols-6 gap-4 text-sm">
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
        </div>

        {tab === 'activity' && (
          <div className="flex items-center gap-2">
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Fecha</span>
            <select
              value={selectedDate}
              onChange={(event) => setSelectedDate(event.target.value)}
              className="rounded-full border border-[#223462] bg-[#111a35] px-3 py-1.5 text-[11px] text-[#dce6ff]"
            >
              {[...(availableDates.length ? availableDates : [dateToday])].map((dateValue) => (
                <option key={dateValue} value={dateValue}>{dateValue}</option>
              ))}
            </select>
            <button
              onClick={handleExportCsv}
              disabled={isExporting}
              className="rounded-full border border-[#00ff88]/40 bg-[#00ff88]/10 px-3 py-1.5 font-mono text-[11px] text-[#00ff88] hover:border-[#00ff88] disabled:opacity-60"
            >
              {isExporting ? 'Exportando...' : 'Export CSV'}
            </button>
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
        </section>
      )}
    </AppShell>
  );
}
