import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import { useNavigate } from 'react-router-dom';
import type { Device, SecurityAlert } from '../types';

export function DashboardPage() {
  const navigate = useNavigate();
  const [devices, setDevices] = useState<Device[]>([]);
  const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    loadDevices();
    // Refresh devices every 30 seconds
    const interval = setInterval(loadDevices, 30000);
    return () => clearInterval(interval);
  }, []);

  const loadDevices = async () => {
    try {
      setIsLoading(true);
      const [devicesData, alertsData] = await Promise.all([
        apiClient.getDevices(),
        apiClient.getAlerts(undefined, false).catch(() => []),
      ]);

      setDevices(devicesData);
      setAlerts(alertsData);
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
      } catch (err) {
        alert('Error al actualizar apodo');
      }
    }
  };

  return (
    <AppShell
      currentPage="dashboard"
      title="Overview"
      subtitle="Centro operativo de dispositivos"
      noScroll
      actions={
        <button
          onClick={loadDevices}
          className="rounded-md border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 font-mono text-xs font-medium tracking-wide text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Sync
        </button>
      }
    >
      <div className="h-[calc(100vh-190px)] grid grid-rows-[minmax(0,1.9fr)_minmax(0,0.75fr)_minmax(0,0.95fr)] gap-3 overflow-hidden">
      <section className="overflow-hidden rounded-2xl border border-[#1a2748] bg-[linear-gradient(160deg,#0d1733_0%,#0b1329_65%,#0a1226_100%)] shadow-[0_12px_30px_rgba(0,0,0,0.35)]">
        <div className="flex items-center gap-2 border-b border-[#1a2748] px-5 py-3">
          <span className="text-[#00d9ff]">▌</span>
          <h2 className="font-display text-base font-bold text-[#e4e6eb]">Dispositivos conocidos</h2>
          <span className="ml-2 rounded-full border border-[#00d9ff]/30 bg-[#00d9ff]/10 px-2 py-0.5 font-mono text-[10px] text-[#00d9ff]">
            {devices.length} nodos
          </span>
        </div>

        {error && (
          <div className="mx-4 mt-3 rounded-xl border border-red-500/30 bg-red-500/10 p-2">
            <p className="font-mono text-[11px] text-red-300">{error}</p>
          </div>
        )}

        <div className="h-[calc(100%-52px)] overflow-auto px-3 py-2">
          <table className="w-full">
            <thead>
              <tr>
                <th className="font-mono text-[9px] uppercase tracking-[0.18em]">Nodo</th>
                <th className="font-mono text-[9px] uppercase tracking-[0.18em]">Dispositivo ID</th>
                <th className="font-mono text-[9px] uppercase tracking-[0.18em]">Estado</th>
                <th className="font-mono text-[9px] uppercase tracking-[0.18em]">Ultimo registro</th>
                <th className="font-mono text-[9px] uppercase tracking-[0.18em]">Consola</th>
              </tr>
            </thead>
            <tbody>
              {isLoading ? (
                <tr>
                  <td colSpan={5} className="py-10 text-center font-mono text-xs text-[#8a97ba]">Cargando dispositivos...</td>
                </tr>
              ) : devices.length === 0 ? (
                <tr>
                  <td colSpan={5} className="py-10 text-center font-mono text-xs text-[#5f6e95]">No hay dispositivos registrados aun.</td>
                </tr>
              ) : (
                devices.map((device) => (
                  <tr key={device.device_id} className="rounded-lg hover:bg-[#101d3f]">
                    <td>
                      <div className="flex items-center gap-2">
                        <span className="text-sm">💻</span>
                        <span className="font-mono text-[10px] text-[#d3deff]">{device.nickname || device.hostname}</span>
                      </div>
                    </td>
                    <td className="font-mono text-[10px] text-[#8ea0cf]">{device.device_id.slice(0, 8)}...{device.device_id.slice(-4)}</td>
                    <td>
                      <div className="flex items-center gap-2">
                        <span className={`inline-block h-2 w-2 rounded-full ${device.online ? 'bg-[#00ff88]' : 'bg-red-400'}`}></span>
                        <span className={`font-mono text-[10px] ${device.online ? 'text-[#00ff88]' : 'text-red-400'}`}>
                          {device.online ? 'ONLINE' : 'OFFLINE'}
                        </span>
                      </div>
                    </td>
                    <td className="font-mono text-[10px] text-[#8ea0cf]">{new Date(device.last_seen).toLocaleString()}</td>
                    <td>
                      <div className="flex gap-2">
                        <button
                          onClick={() => handleUpdateNickname(device.device_id, device.nickname)}
                          className="rounded-lg border border-[#00d9ff]/30 bg-[#00d9ff]/10 px-2 py-1 font-mono text-[9px] text-[#00d9ff]"
                        >
                          Editar
                        </button>
                        <button
                          onClick={() => navigate(`/devices/${device.device_id}`)}
                          className="rounded-lg border border-[#00ff88]/30 bg-[#00ff88]/10 px-2 py-1 font-mono text-[9px] text-[#00ff88]"
                        >
                          Abrir
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

      <section className="grid grid-cols-6 gap-2 overflow-hidden">
        {[
          { label: 'Dispositivos en vivo', value: devices.filter((d) => d.online).length.toString(), note: 'ultimos 3 min', color: '#00ff88' },
          { label: 'Activos hoy', value: devices.length.toString(), note: 'dispositivos', color: '#00d9ff' },
          { label: 'Tiempo activo hoy', value: '-', note: 'global', color: '#00d9ff' },
          { label: 'Tiempo inactivo hoy', value: '-', note: 'global', color: '#ff9f1a' },
          { label: 'Alertas nuevas', value: alerts.length.toString(), note: 'sin resolver', color: '#ffd54a' },
          { label: 'Estado general', value: alerts.length > 0 ? 'Riesgo' : 'OK', note: 'todos los dispositivos', color: alerts.length > 0 ? '#ff5f7a' : '#8f7bff' },
        ].map((kpi) => (
          <article key={kpi.label} className="rounded-xl border border-[#1a2748] bg-[linear-gradient(145deg,#0f1a37,#0b1329)] px-3 py-2 min-h-[76px] shadow-[0_8px_20px_rgba(0,0,0,0.3)]">
            <p className="font-mono text-[9px] uppercase leading-tight tracking-[0.18em] text-[#7080aa]">{kpi.label}</p>
            <p className="mt-2 font-display text-[18px] font-bold leading-none" style={{ color: kpi.color }}>{kpi.value}</p>
            <p className="mt-1 font-mono text-[9px] leading-tight text-[#5f6e95]">{kpi.note}</p>
          </article>
        ))}
      </section>

      <section className="grid grid-cols-2 gap-3 overflow-hidden">
        <article className="rounded-2xl border border-[#1a2748] bg-[linear-gradient(150deg,#0d1733,#0b1329)] px-4 py-4 min-h-0 shadow-[0_10px_24px_rgba(0,0,0,0.32)]">
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#7080aa]">Dispositivos en vivo</p>
          <div className="mt-3 h-[calc(100%-26px)] space-y-2 overflow-auto">
            {devices.filter((d) => d.online).slice(0, 8).map((d) => (
              <button
                key={d.device_id}
                onClick={() => navigate(`/devices/${d.device_id}`)}
                className="flex w-full items-center justify-between rounded-md border border-[#1a2748] bg-[#0a1022] px-3 py-2 text-left hover:border-[#00d9ff]/50"
              >
                <span className="font-mono text-xs text-[#c8d5ff]">{d.nickname || d.hostname}</span>
                <span className="font-mono text-[10px] text-[#00ff88]">ONLINE</span>
              </button>
            ))}
            {devices.filter((d) => d.online).length === 0 && (
              <p className="font-mono text-xs text-[#5f6e95]">No hay dispositivos online.</p>
            )}
          </div>
        </article>

        <article className="rounded-2xl border border-[#1a2748] bg-[linear-gradient(150deg,#0d1733,#0b1329)] px-4 py-4 min-h-0 shadow-[0_10px_24px_rgba(0,0,0,0.32)]">
          <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#7080aa]">Alertas</p>
          <div className="mt-3 h-[calc(100%-26px)] space-y-2 overflow-auto">
            {alerts.slice(0, 6).map((alert) => (
              <div key={alert.id} className="rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2">
                <p className="font-mono text-xs text-red-200">{alert.alert_type}</p>
                <p className="mt-1 font-mono text-[10px] text-red-200/80">{alert.description}</p>
              </div>
            ))}
            {alerts.length === 0 && (
              <p className="font-mono text-xs text-[#5f6e95]">No hay alertas activas.</p>
            )}
          </div>
        </article>
      </section>
      </div>

      <section className="cyber-card overflow-hidden rounded-xl hidden">
        <div className="flex items-center justify-between border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <div>
            <h2 className="font-display text-2xl font-bold text-[#e4e6eb]">Estado de Seguridad</h2>
            <p className="mt-1 text-sm text-[#a0a5b2]">Alertas no resueltas detectadas en la red</p>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-2xl">{alerts.length > 0 ? '🚨' : '🛡'}</span>
            <span className={`rounded-full border px-4 py-1 text-sm font-bold ${alerts.length > 0 ? 'border-red-500/60 bg-red-500/15 text-red-300' : 'border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]'}`}>
              {alerts.length > 0 ? `${alerts.length} nuevas` : 'Sin novedades'}
            </span>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-3 p-4">
          {alerts.length === 0 ? (
            <p className="rounded-lg border border-[#1e2339] bg-[#0a0e27] px-4 py-3 text-sm text-[#a0a5b2]">
              No hay alertas activas por ahora.
            </p>
          ) : (
            alerts.slice(0, 4).map((alert) => (
              <div key={alert.id} className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3">
                <p className="text-sm font-semibold text-red-300">{alert.alert_type}</p>
                <p className="mt-1 text-xs text-red-200/90">{alert.description}</p>
              </div>
            ))
          )}
        </div>
      </section>
    </AppShell>
  );
}
