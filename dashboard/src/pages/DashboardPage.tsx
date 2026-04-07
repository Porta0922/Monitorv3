import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { Device, SecurityAlert } from '../types';

interface LiveDeviceItem {
  device_id: string;
  app: string;
  title: string;
  last_seen: string;
  ago_sec: number;
  is_live: boolean;
  is_stale?: boolean;
  is_idle: boolean;
  duration: string;
}

export function DashboardPage() {
  const navigate = useNavigate();
  const [devices, setDevices] = useState<Device[]>([]);
  const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
  const [liveDevices, setLiveDevices] = useState<LiveDeviceItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');
  const [deviceFilter, setDeviceFilter] = useState('');

  const formatDuration = (seconds?: number) => {
    const safeSeconds = Math.max(0, seconds || 0);
    const hours = Math.floor(safeSeconds / 3600);
    const minutes = Math.floor((safeSeconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  const totalActiveSeconds = devices.reduce((acc, device) => acc + (device.active_time_today_seconds || 0), 0);
  const totalIdleSeconds = devices.reduce((acc, device) => acc + (device.idle_time_today_seconds || 0), 0);
  const onlineDevices = devices.filter((device) => device.online);

  const nameByDeviceId = useMemo(
    () => new Map(devices.map((d) => [d.device_id, d.nickname || d.hostname])),
    [devices]
  );

  const getNodeName = (device: Device) => {
    return device.nickname || device.hostname || `${device.device_id.slice(0, 8)}...`;
  };

  useEffect(() => {
    loadDevices();
    const interval = setInterval(loadDevices, 30000);
    return () => clearInterval(interval);
  }, []);

  const loadDevices = async () => {
    try {
      setIsLoading(true);
      const [devicesData, alertsData, liveData] = await Promise.all([
        apiClient.getDevices(),
        apiClient.getAlerts(undefined, false).catch(() => []),
        apiClient.getLiveDevices().catch(() => []),
      ]);

      setDevices(devicesData);
      setAlerts(alertsData);
      setLiveDevices(liveData as LiveDeviceItem[]);
      setError('');
    } catch (err: any) {
      setError(err.message || 'Error al cargar dispositivos');
      console.error('Error loading devices:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleUpdateNickname = async (deviceId: string, currentNickname?: string) => {
    const nickname = prompt('Ingresar apodo del dispositivo:', currentNickname || '');
    if (nickname !== null) {
      try {
        await apiClient.updateDevice(deviceId, nickname);
        loadDevices();
      } catch {
        alert('Error al actualizar apodo');
      }
    }
  };

  return (
    <AppShell
      currentPage="dashboard"
      title="Centro de Mando AME"
      subtitle="Visibilidad operativa de actividad por dispositivo"
      noScroll
      actions={
        <button
          onClick={loadDevices}
          className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-4 py-2 font-mono text-xs font-semibold tracking-wide text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      <div className="h-[calc(100vh-190px)] grid grid-rows-[minmax(0,0.9fr)_minmax(0,2.2fr)_minmax(0,1.2fr)] gap-4 overflow-hidden">
        <section className="grid grid-cols-6 gap-3">
          {[
            { label: 'Dispositivos', value: devices.length.toString(), note: 'registrados', color: '#00d9ff' },
            { label: 'En linea', value: onlineDevices.length.toString(), note: 'ultimos minutos', color: '#00ff88' },
            { label: 'Activo hoy', value: formatDuration(totalActiveSeconds), note: 'suma global', color: '#00d9ff' },
            { label: 'Inactivo hoy', value: formatDuration(totalIdleSeconds), note: 'suma global', color: '#ff9f1a' },
            { label: 'Alertas abiertas', value: alerts.length.toString(), note: 'sin resolver', color: '#ffd54a' },
            { label: 'Riesgo', value: alerts.length > 0 ? 'Atencion' : 'Normal', note: 'estado operativo', color: alerts.length > 0 ? '#ff5f7a' : '#8f7bff' },
          ].map((kpi) => (
            <article
              key={kpi.label}
              className="rounded-2xl border border-[#1a2748] bg-[radial-gradient(circle_at_top_left,#132554,#0b1329_65%)] px-4 py-3 shadow-[0_10px_22px_rgba(0,0,0,0.35)]"
            >
              <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#8ea0cf]">{kpi.label}</p>
              <p className="mt-2 font-display text-[22px] font-bold leading-none" style={{ color: kpi.color }}>
                {kpi.value}
              </p>
              <p className="mt-1 font-mono text-[10px] text-[#6f82b1]">{kpi.note}</p>
            </article>
          ))}
        </section>

        <section className="overflow-hidden rounded-2xl border border-[#1a2748] bg-[linear-gradient(165deg,#0f1d43_0%,#0b1329_70%)] shadow-[0_14px_30px_rgba(0,0,0,0.35)]">
          <div className="flex flex-col gap-2 border-b border-[#20315a] px-5 py-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <h2 className="font-display text-base font-bold text-[#e4e6eb]">Dispositivos Conocidos</h2>
                <span className="rounded-full border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-2.5 py-0.5 font-mono text-[10px] text-[#00d9ff]">
                  {devices.length} nodos
                </span>
              </div>
              <p className="font-mono text-[11px] text-[#7f93c7]">Activo/Inactivo medido por teclado y mouse</p>
            </div>
            <div className="relative max-w-sm">
              <span className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-[#4a5d8a]">
                <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                  <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
                </svg>
              </span>
              <input
                type="text"
                value={deviceFilter}
                onChange={e => setDeviceFilter(e.target.value)}
                placeholder="Buscar por MAC, hostname, apodo o Device ID…"
                className="w-full rounded-lg border border-[#1e2d52] bg-[#080f26] py-1.5 pl-8 pr-3 font-mono text-[11px] text-[#c5d3f0] placeholder-[#3d4f73] focus:border-[#00d9ff]/60 focus:outline-none"
              />
              {deviceFilter && (
                <button
                  onClick={() => setDeviceFilter('')}
                  className="absolute inset-y-0 right-2 flex items-center font-mono text-[11px] text-[#4a5d8a] hover:text-[#c5d3f0]"
                >
                  ✕
                </button>
              )}
            </div>
          </div>

          {error && (
            <div className="mx-4 mt-3 rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2">
              <p className="font-mono text-[11px] text-red-300">{error}</p>
            </div>
          )}

          <div className="h-[calc(100%-56px)] overflow-auto px-3 py-2">
            <table>
              <thead>
                <tr>
                  <th>Nodo</th>
                  <th>Device ID</th>
                  <th>Estado</th>
                  <th>Activo hoy</th>
                  <th>Inactivo hoy</th>
                  <th>Ultima senal</th>
                  <th>Acciones</th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  <tr>
                    <td colSpan={7} className="py-10 text-center font-mono text-xs text-[#8a97ba]">
                      Cargando dispositivos...
                    </td>
                  </tr>
                ) : devices.length === 0 ? (
                  <tr>
                    <td colSpan={7} className="py-10 text-center font-mono text-xs text-[#5f6e95]">
                      No hay dispositivos registrados aun.
                    </td>
                  </tr>
                ) : (
                  devices
                  .filter(d => {
                    if (!deviceFilter.trim()) return true;
                    const q = deviceFilter.trim().toLowerCase();
                    return (
                      d.mac_address?.toLowerCase().includes(q) ||
                      d.device_id.toLowerCase().includes(q) ||
                      d.hostname?.toLowerCase().includes(q) ||
                      (d.nickname ?? '').toLowerCase().includes(q)
                    );
                  })
                  .map((device) => (
                    <tr key={device.device_id}>
                      <td>
                        <div className="flex flex-col">
                          <span className="font-display text-sm text-[#dce6ff]">{getNodeName(device)}</span>
                          <span className="font-mono text-[10px] text-[#7387bc]">{device.hostname}</span>
                        </div>
                      </td>
                      <td>
                        <div className="flex flex-col gap-0.5">
                          <span className="font-mono text-[10px] text-[#8ea0cf]">{device.device_id.slice(0, 8)}…{device.device_id.slice(-4)}</span>
                          {device.mac_address && (
                            <span className="font-mono text-[10px] text-[#4e6bab]">{device.mac_address}</span>
                          )}
                        </div>
                      </td>
                      <td>
                        <div className="flex items-center gap-2">
                          <span
                            className={`inline-flex rounded-full px-2.5 py-1 font-mono text-[10px] ${
                              device.online
                                ? 'border border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]'
                                : 'border border-red-500/40 bg-red-500/10 text-red-300'
                            }`}
                          >
                            {device.online ? 'ONLINE' : 'OFFLINE'}
                          </span>
                          {device.stale && (
                            <span className="inline-flex rounded-full border border-red-500/40 bg-red-500/10 px-2.5 py-1 font-mono text-[10px] text-red-300">
                              STALE
                            </span>
                          )}
                        </div>
                      </td>
                      <td className="font-mono text-[11px] text-[#00d9ff]">{formatDuration(device.active_time_today_seconds)}</td>
                      <td className="font-mono text-[11px] text-[#ff9f1a]">{formatDuration(device.idle_time_today_seconds)}</td>
                      <td className="font-mono text-[10px] text-[#8ea0cf]">{new Date(device.last_seen).toLocaleString()}</td>
                      <td>
                        <div className="flex gap-2">
                          <button
                            onClick={() => handleUpdateNickname(device.device_id, device.nickname)}
                            className="rounded-full border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-3 py-1.5 font-mono text-[10px] text-[#00d9ff] hover:border-[#00d9ff]"
                          >
                            Editar
                          </button>
                          <button
                            onClick={() => navigate(`/devices/${device.device_id}`)}
                            className="rounded-full border border-[#00ff88]/40 bg-[#00ff88]/10 px-3 py-1.5 font-mono text-[10px] text-[#00ff88] hover:border-[#00ff88]"
                          >
                            Abrir consola
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

        <section className="grid grid-cols-2 gap-4 overflow-hidden">
          <article className="rounded-2xl border border-[#1a2748] bg-[linear-gradient(155deg,#0f1d43,#0b1329)] px-4 py-4 min-h-0 shadow-[0_10px_24px_rgba(0,0,0,0.32)]">
            <div className="mb-3 flex items-center justify-between">
              <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#8ea0cf]">Live Devices</p>
              <span className="rounded-full border border-[#00ff88]/40 bg-[#00ff88]/10 px-2 py-0.5 font-mono text-[10px] text-[#00ff88]">
                {liveDevices.filter((d) => d.is_live).length}
              </span>
            </div>
            <div className="space-y-2 overflow-auto">
              {liveDevices.slice(0, 7).map((live) => (
                <button
                  key={live.device_id}
                  onClick={() => navigate(`/devices/${live.device_id}`)}
                  className="flex w-full items-center justify-between rounded-xl border border-[#21325d] bg-[#0a122a] px-3 py-2 text-left hover:border-[#00d9ff]/60"
                >
                  <div className="min-w-0">
                    <p className="truncate font-mono text-xs text-[#dce6ff]">{nameByDeviceId.get(live.device_id) || `${live.device_id.slice(0, 8)}...`}</p>
                    <p className="truncate font-mono text-[10px] text-[#8ea0cf]">{live.app}</p>
                  </div>
                  <span className={`rounded-full px-2 py-1 font-mono text-[10px] ${live.is_stale ? 'text-red-400' : (live.is_live ? 'text-[#00ff88]' : 'text-[#ff9f1a]')}`}>
                    {live.is_stale ? 'STALE' : (live.is_live ? 'LIVE' : `${live.ago_sec}s`)}
                  </span>
                </button>
              ))}
              {liveDevices.length === 0 && <p className="font-mono text-xs text-[#5f6e95]">No hay telemetria en vivo.</p>}
            </div>
          </article>

          <article className="rounded-2xl border border-[#1a2748] bg-[linear-gradient(155deg,#0f1d43,#0b1329)] px-4 py-4 min-h-0 shadow-[0_10px_24px_rgba(0,0,0,0.32)]">
            <div className="mb-3 flex items-center justify-between">
              <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#8ea0cf]">Alertas activas</p>
              <span className="rounded-full border border-[#ff5f7a]/40 bg-[#ff5f7a]/10 px-2 py-0.5 font-mono text-[10px] text-[#ff8ea0]">
                {alerts.length}
              </span>
            </div>
            <div className="space-y-2 overflow-auto">
              {alerts.slice(0, 6).map((alert) => (
                <div key={alert.id} className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2">
                  <p className="font-mono text-xs text-red-200">{alert.alert_type}</p>
                  <p className="mt-1 line-clamp-2 font-mono text-[10px] text-red-200/80">{alert.description}</p>
                </div>
              ))}
              {alerts.length === 0 && <p className="font-mono text-xs text-[#5f6e95]">No hay alertas activas.</p>}
            </div>
          </article>
        </section>
      </div>
    </AppShell>
  );
}
