import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { Device } from '../types';

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

export function ActivityPage() {
  const navigate = useNavigate();
  const [devices, setDevices] = useState<Device[]>([]);
  const [liveDevices, setLiveDevices] = useState<LiveDeviceItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');
  const [lastUpdatedAt, setLastUpdatedAt] = useState<string | null>(null);

  useEffect(() => {
    loadLiveSnapshot();
    const interval = setInterval(loadLiveSnapshot, 10000);
    return () => clearInterval(interval);
  }, []);

  const nameByDeviceId = useMemo(
    () => new Map(devices.map((d) => [d.device_id, d.nickname || d.hostname])),
    [devices]
  );

  const currentByDevice = useMemo(() => {
    const byDevice = new Map<string, LiveDeviceItem>();

    for (const row of liveDevices) {
      const existing = byDevice.get(row.device_id);
      if (!existing || new Date(row.last_seen).getTime() > new Date(existing.last_seen).getTime()) {
        byDevice.set(row.device_id, row);
      }
    }

    return Array.from(byDevice.values()).sort(
      (a, b) => new Date(b.last_seen).getTime() - new Date(a.last_seen).getTime()
    );
  }, [liveDevices]);

  const getNodeName = (deviceId: string) => {
    return nameByDeviceId.get(deviceId) || `${deviceId.slice(0, 8)}...`;
  };

  const loadLiveSnapshot = async () => {
    try {
      setIsLoading(true);
      const [devicesData, liveData] = await Promise.all([
        apiClient.getDevices().catch(() => [] as Device[]),
        apiClient.getLiveDevices({ limit: 100 }).catch(() => []),
      ]);
      setDevices(devicesData);
      setLiveDevices(liveData as LiveDeviceItem[]);
      setLastUpdatedAt(new Date().toISOString());
      setError('');
    } catch (err) {
      setError('No se pudo cargar la telemetria en vivo. Revisa conexion API y servicio server.');
      console.error('Error loading live snapshot:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="activity"
      title="En Vivo"
      subtitle="Ultima aplicacion detectada por nodo"
      noScroll
      actions={
        <div className="flex items-center gap-3">
          <span className="font-mono text-[10px] text-[#5a6a90]">
            {lastUpdatedAt ? `Actualizado ${new Date(lastUpdatedAt).toLocaleTimeString()}` : ''}
          </span>
          <button
            onClick={loadLiveSnapshot}
            className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-3 py-1.5 font-mono text-[10px] text-[#00d9ff] hover:border-[#00d9ff]"
          >
            Actualizar
          </button>
        </div>
      }
    >
      <div className="h-[calc(100vh-190px)] overflow-hidden rounded-2xl border border-[#1a2748] bg-[linear-gradient(165deg,#0f1d43,#0b1329)] shadow-[0_14px_30px_rgba(0,0,0,0.35)] flex flex-col">
        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-[#20315a] px-5 py-3">
          <div className="flex items-center gap-3">
            <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#8ea0cf]">Nodos activos</p>
            <span className="rounded-full border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-2.5 py-0.5 font-mono text-[10px] text-[#00d9ff]">
              {currentByDevice.length} nodos
            </span>
          </div>
          {error && <p className="font-mono text-[10px] text-red-300">{error}</p>}
        </div>

        {/* Table */}
        <div className="flex-1 overflow-auto">
          {isLoading ? (
            <p className="py-10 text-center font-mono text-[11px] text-[#5a6a90]">Cargando telemetria...</p>
          ) : currentByDevice.length === 0 ? (
            <p className="py-10 text-center font-mono text-[11px] text-[#5a6a90]">Sin telemetria en vivo. Verifica que el agente este ejecutandose.</p>
          ) : (
            <table className="w-full border-collapse">
              <thead>
                <tr className="sticky top-0 z-10 border-b border-[#20315a] bg-[#0a122a]">
                  <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Nodo</th>
                  <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Estado</th>
                  <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Aplicacion</th>
                  <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Ventana activa</th>
                  <th className="px-5 py-2.5 text-right font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Hace</th>
                </tr>
              </thead>
              <tbody>
                {currentByDevice.map((live) => (
                  <tr
                    key={live.device_id}
                    onClick={() => navigate(`/devices/${live.device_id}`)}
                    className="cursor-pointer border-b border-[#1a2748] hover:bg-[#0f1c3a]"
                  >
                    <td className="px-5 py-2.5">
                      <p className="font-mono text-[11px] text-[#dce6ff]">{getNodeName(live.device_id)}</p>
                    </td>
                    <td className="px-5 py-2.5">
                      <span className={`inline-flex rounded-full px-2.5 py-0.5 font-mono text-[10px] ${
                        live.is_stale
                          ? 'border border-red-500/40 bg-red-500/10 text-red-300'
                          : live.is_idle
                          ? 'border border-[#ff9f1a]/40 bg-[#ff9f1a]/10 text-[#ff9f1a]'
                          : 'border border-[#00ff88]/40 bg-[#00ff88]/10 text-[#00ff88]'
                      }`}>
                        {live.is_stale ? 'STALE' : live.is_idle ? 'IDLE' : 'ACTIVE'}
                      </span>
                    </td>
                    <td className="px-5 py-2.5 font-mono text-[11px] text-[#dce6ff]">{live.app || '—'}</td>
                    <td className="max-w-[380px] truncate px-5 py-2.5 font-mono text-[10px] text-[#8ea0cf]">{live.title || '(sin titulo)'}</td>
                    <td className="px-5 py-2.5 text-right font-mono text-[11px] text-[#00d9ff]">{live.ago_sec}s</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </AppShell>
  );
}

