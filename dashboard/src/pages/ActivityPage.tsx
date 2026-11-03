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
        apiClient.getDevices(),
        apiClient.getLiveDevices({ limit: 100 }),
      ]);
      setDevices(devicesData);
      setLiveDevices(liveData as LiveDeviceItem[]);
      setLastUpdatedAt(new Date().toISOString());
      setError('');
    } catch (err) {
      setError('No se pudo actualizar la actividad. Los datos anteriores pueden estar desactualizados.');
      console.error('Error loading live snapshot:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="activity"
      title="Actividad reciente"
      subtitle="Última aplicación reportada por equipo · hasta 100 registros por consulta."
      noScroll
      actions={
        <div className="flex items-center gap-3">
          <span className="font-sans text-[12px] text-[var(--palette-53685c)]">
            {lastUpdatedAt ? `Actualizado ${new Date(lastUpdatedAt).toLocaleTimeString()}` : ''}
          </span>
          <button
            onClick={loadLiveSnapshot}
            className="rounded-full border border-[var(--palette-dfe7e3)]/50 bg-[var(--palette-17715a)]/10 px-3 py-1.5 font-sans text-[12px] text-[var(--palette-17715a)] hover:border-[var(--palette-dfe7e3)]"
          >
            Actualizar
          </button>
        </div>
      }
    >
      <div className="min-h-[calc(100vh-190px)] overflow-hidden rounded-2xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] shadow-sm flex flex-col">
        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-[var(--palette-dfe7e3)] px-5 py-3">
          <div className="flex items-center gap-3">
            <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Equipos con actividad reportada</p>
            <span className="rounded-full border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-17715a)]/10 px-2.5 py-0.5 font-sans text-[12px] text-[var(--palette-17715a)]">
              {currentByDevice.length} nodos
            </span>
          </div>
          {error && <p className="font-sans text-[12px] text-red-700">{error}</p>}
        </div>

        {/* Table */}
        <div className="flex-1 overflow-auto">
          {isLoading ? (
            <p className="py-10 text-center font-sans text-[12px] text-[var(--palette-53685c)]">Cargando telemetria...</p>
          ) : currentByDevice.length === 0 ? (
            <p className="py-10 text-center font-sans text-[12px] text-[var(--palette-53685c)]">Sin telemetria en vivo. Verifica que el agente este ejecutandose.</p>
          ) : (
            <table className="w-full border-collapse">
              <thead>
                <tr className="sticky top-0 z-10 border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
                  <th className="px-5 py-2.5 text-left font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Nodo</th>
                  <th className="px-5 py-2.5 text-left font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Estado</th>
                  <th className="px-5 py-2.5 text-left font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Aplicacion</th>
                  <th className="px-5 py-2.5 text-left font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Última ventana reportada</th>
                  <th className="px-5 py-2.5 text-right font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Hace</th>
                </tr>
              </thead>
              <tbody>
                {currentByDevice.map((live) => (
                  <tr
                    key={live.device_id}
                    onClick={() => navigate(`/devices/${live.device_id}`)}
                    className="cursor-pointer border-b border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-f5f7f6)]"
                  >
                    <td className="px-5 py-2.5">
                      <p className="font-sans text-[12px] text-[var(--palette-53685c)]">{getNodeName(live.device_id)}</p>
                    </td>
                    <td className="px-5 py-2.5">
                      <span className={`inline-flex rounded-full px-2.5 py-0.5 font-sans text-[12px] ${
                        live.is_stale
                          ? 'border border-red-500/40 bg-red-500/10 text-red-700'
                          : live.is_idle
                          ? 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-97621b)]/10 text-[var(--palette-97621b)]'
                          : 'border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 text-[var(--palette-31764d)]'
                      }`}>
                        {live.is_stale ? 'Sin señal reciente' : 'Reporte reciente'}
                      </span>
                    </td>
                    <td className="px-5 py-2.5 font-sans text-[12px] text-[var(--palette-53685c)]">{live.app || '—'}</td>
                    <td className="max-w-[380px] truncate px-5 py-2.5 font-sans text-[12px] text-[var(--palette-53685c)]">{live.title || '(sin titulo)'}</td>
                    <td className="px-5 py-2.5 text-right font-sans text-[12px] text-[var(--palette-17715a)]">{live.ago_sec}s</td>
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

