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

  const tabClass = (key: TabKey) =>
    `rounded-lg border px-4 py-2 text-sm font-semibold transition-all ${
      tab === key
        ? 'border-[#00d9ff] bg-[#00d9ff]/15 text-[#00d9ff]'
        : 'border-[#1e2339] bg-[#131829] text-[#a0a5b2] hover:border-[#00d9ff]/40 hover:text-[#e4e6eb]'
    }`;

  return (
    <AppShell
      currentPage="dashboard"
      title={device ? `Consola: ${device.nickname || device.hostname}` : 'Consola de Computadora'}
      subtitle={deviceId || 'Sin dispositivo'}
      actions={
        <button
          onClick={() => navigate('/dashboard')}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Volver al Nexus
        </button>
      }
    >
      {device && (
        <section className="cyber-card rounded-xl p-5">
          <div className="grid grid-cols-4 gap-4 text-sm">
            <div>
              <p className="text-[#717579]">Hostname</p>
              <p className="font-display text-base text-[#e4e6eb]">{device.hostname}</p>
            </div>
            <div>
              <p className="text-[#717579]">Estado</p>
              <p className={device.online ? 'text-[#00ff88]' : 'text-red-400'}>{device.online ? 'ONLINE' : 'OFFLINE'}</p>
            </div>
            <div>
              <p className="text-[#717579]">MAC</p>
              <p className="font-mono text-[#a0a5b2]">{device.mac_address || 'N/A'}</p>
            </div>
            <div>
              <p className="text-[#717579]">Ultima señal</p>
              <p className="text-[#a0a5b2]">{new Date(device.last_seen).toLocaleString()}</p>
            </div>
          </div>
        </section>
      )}

      <section className="flex gap-3">
        <button className={tabClass('activity')} onClick={() => setTab('activity')}>
          Actividad ({tabStats.activity})
        </button>
        <button className={tabClass('inventory')} onClick={() => setTab('inventory')}>
          Inventario ({tabStats.inventory})
        </button>
        <button className={tabClass('usb')} onClick={() => setTab('usb')}>
          USB ({tabStats.usb})
        </button>
      </section>

      {tab === 'activity' && (
        <section className="flex items-center gap-2">
          <span className="text-xs uppercase tracking-[0.2em] text-[#717579]">Ventana</span>
          {(['1h', '24h', '7d', '30d'] as ActivityWindow[]).map((windowKey) => (
            <button
              key={windowKey}
              onClick={() => setActivityWindow(windowKey)}
              className={`rounded-md border px-3 py-1.5 text-xs font-semibold transition-all ${
                activityWindow === windowKey
                  ? 'border-[#00d9ff] bg-[#00d9ff]/15 text-[#00d9ff]'
                  : 'border-[#1e2339] bg-[#131829] text-[#a0a5b2] hover:border-[#00d9ff]/40 hover:text-[#e4e6eb]'
              }`}
            >
              {windowKey}
            </button>
          ))}
        </section>
      )}

      {error && <section className="rounded-xl border border-red-500/30 bg-red-500/10 px-5 py-4 text-sm text-red-300">{error}</section>}

      {isLoading ? (
        <section className="cyber-card rounded-xl px-6 py-10 text-center text-[#a0a5b2]">Cargando telemetria...</section>
      ) : (
        <section className="cyber-card overflow-hidden rounded-xl">
          {tab === 'activity' && (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr>
                    <th>Timestamp</th>
                    <th>Aplicacion</th>
                    <th>Ventana</th>
                    <th>Duracion</th>
                  </tr>
                </thead>
                <tbody>
                  {activity.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="text-center text-[#a0a5b2]">No hay logs de actividad.</td>
                    </tr>
                  ) : (
                    activity.map((log, idx) => (
                      <tr key={`${log.timestamp}-${idx}`}>
                        <td>{new Date(log.timestamp).toLocaleString()}</td>
                        <td className="text-[#e4e6eb]">{log.app_name}</td>
                        <td className="max-w-[760px] truncate text-[#a0a5b2]">{log.window_title}</td>
                        <td className="font-mono text-[#00ff88]">{log.duration_seconds}s</td>
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
                      <td colSpan={4} className="text-center text-[#a0a5b2]">
                        Sin datos de inventario para este equipo o endpoint no disponible.
                      </td>
                    </tr>
                  ) : (
                    inventory.map((app, idx) => (
                      <tr key={`${app.app_name}-${idx}`}>
                        <td className="text-[#e4e6eb]">{app.app_name}</td>
                        <td>{app.version || 'Unknown'}</td>
                        <td>
                          <span className={app.verified ? 'text-[#00ff88]' : 'text-red-400'}>
                            {app.verified ? 'VERIFIED' : 'UNVERIFIED'}
                          </span>
                        </td>
                        <td className="max-w-[560px] truncate font-mono text-xs">{app.exe_hash}</td>
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
                    <th>Timestamp</th>
                    <th>Nombre</th>
                    <th>Serial</th>
                    <th>Accion</th>
                  </tr>
                </thead>
                <tbody>
                  {usbEvents.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="text-center text-[#a0a5b2]">
                        Sin eventos USB para este equipo o endpoint no disponible.
                      </td>
                    </tr>
                  ) : (
                    usbEvents.map((event, idx) => (
                      <tr key={`${event.timestamp}-${idx}`}>
                        <td>{new Date(event.timestamp).toLocaleString()}</td>
                        <td className="text-[#e4e6eb]">{event.device_name}</td>
                        <td className="font-mono text-xs text-[#a0a5b2]">{event.serial_number}</td>
                        <td className={event.action === 'IN' ? 'text-[#00ff88]' : 'text-red-400'}>
                          {event.action === 'IN' ? 'CONNECTED' : 'DISCONNECTED'}
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
