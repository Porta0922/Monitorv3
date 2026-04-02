import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { ActivityLog, AppInfo, Device, USBEvent } from '../types';

type TabKey = 'activity' | 'inventory' | 'usb';
type ActivityWindow = '1h' | '24h' | '7d' | '30d';

const activityWindowToHours: Record<ActivityWindow, number> = {
  '1h': 1,
  '24h': 24,
  '7d': 24 * 7,
  '30d': 24 * 30,
};

export function DeviceDetailPage() {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();

  const [device, setDevice] = useState<Device | null>(null);
  const [activity, setActivity] = useState<ActivityLog[]>([]);
  const [inventory, setInventory] = useState<AppInfo[]>([]);
  const [usbEvents, setUsbEvents] = useState<USBEvent[]>([]);
  const [tab, setTab] = useState<TabKey>('activity');
  const [activityWindow, setActivityWindow] = useState<ActivityWindow>('24h');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!deviceId) return;

    const load = async () => {
      setIsLoading(true);
      setError('');

      try {
        const [devices, logs] = await Promise.all([
          apiClient.getDevices(),
          apiClient.getActivityLogs(deviceId, {
            limit: 500,
            hours: activityWindowToHours[activityWindow],
          }),
        ]);

        const selected = devices.find((d) => d.device_id === deviceId) || null;
        setDevice(selected);
        setActivity(logs);

        try {
          const apps = await apiClient.getApps(deviceId);
          setInventory(apps);
        } catch {
          setInventory([]);
        }

        try {
          const usb = await apiClient.getUsbHistory(deviceId, 250);
          setUsbEvents(usb);
        } catch {
          setUsbEvents([]);
        }
      } catch (err: any) {
        setError(err?.message || 'No fue posible cargar la consola de dispositivo.');
      } finally {
        setIsLoading(false);
      }
    };

    load();
  }, [deviceId, activityWindow]);

  const tabStats = useMemo(
    () => ({
      activity: activity.length,
      inventory: inventory.length,
      usb: usbEvents.length,
    }),
    [activity.length, inventory.length, usbEvents.length]
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
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Ventana</span>
            {(['1h', '24h', '7d', '30d'] as ActivityWindow[]).map((windowKey) => (
              <button
                key={windowKey}
                onClick={() => setActivityWindow(windowKey)}
                className={`rounded-full border px-3 py-1.5 text-[11px] font-semibold transition-all ${
                  activityWindow === windowKey
                    ? 'border-[#00d9ff] bg-[#00d9ff]/15 text-[#00d9ff]'
                    : 'border-[#223462] bg-[#111a35] text-[#8ea0cf] hover:border-[#00d9ff]/50 hover:text-[#dce6ff]'
                }`}
              >
                {windowKey}
              </button>
            ))}
          </div>
        )}
      </section>

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
                    <th>Hora</th>
                    <th>Aplicacion</th>
                    <th>Ventana</th>
                    <th>Duracion</th>
                  </tr>
                </thead>
                <tbody>
                  {activity.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="py-8 text-center text-[#8fa0c9]">No hay logs de actividad.</td>
                    </tr>
                  ) : (
                    activity.map((log, idx) => (
                      <tr key={`${log.timestamp}-${idx}`}>
                        <td className="whitespace-nowrap font-mono text-[11px] text-[#9eb0dc]">{new Date(log.timestamp).toLocaleString()}</td>
                        <td className="max-w-[320px] truncate font-mono text-[12px] text-[#dce6ff]" title={log.app_name}>
                          {shortAppName(log.app_name)}
                        </td>
                        <td className="max-w-[540px] truncate text-[12px] text-[#9eb0dc]" title={log.window_title}>
                          {log.window_title || 'Sin titulo'}
                        </td>
                        <td className="whitespace-nowrap font-mono text-[11px] text-[#00ff88]">{formatDuration(log.duration_seconds)}</td>
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
