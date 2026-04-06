import { useEffect, useMemo, useState } from 'react';
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
  is_idle: boolean;
  duration: string;
}

export function ActivityPage() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [liveDevices, setLiveDevices] = useState<LiveDeviceItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);

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

  const loadLiveSnapshot = async () => {
    try {
      setIsLoading(true);
      const [devicesData, liveData] = await Promise.all([
        apiClient.getDevices(),
        apiClient.getLiveDevices().catch(() => []),
      ]);
      setDevices(devicesData);
      setLiveDevices(liveData as LiveDeviceItem[]);
    } catch (err) {
      console.error('Error loading live snapshot:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="activity"
      title="En Vivo"
      subtitle="Ultima aplicacion detectada por nodo (estado actual)"
      actions={
        <button
          onClick={loadLiveSnapshot}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl overflow-hidden">
        <div className="border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <h2 className="text-lg font-semibold text-[#e4e6eb]">Estado actual por nodo ({currentByDevice.length})</h2>
        </div>

        {isLoading ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Cargando estado en vivo...</div>
        ) : currentByDevice.length === 0 ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">No hay telemetria en vivo.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Nodo</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Estado</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Aplicacion</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Ventana</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Hace</th>
                </tr>
              </thead>
              <tbody>
                {currentByDevice.map((live) => (
                  <tr key={live.device_id} className="border-b border-[#1e2339] hover:bg-[#131829]">
                    <td className="px-6 py-3 font-mono text-xs text-[#a0a5b2]">
                      {nameByDeviceId.get(live.device_id) || `${live.device_id.slice(0, 8)}...`}
                    </td>
                    <td className="px-6 py-3">
                      <span className={`rounded-full px-2 py-1 font-mono text-[10px] ${live.is_idle ? 'text-[#ff9f1a]' : 'text-[#00ff88]'}`}>
                        {live.is_idle ? 'IDLE' : 'ACTIVE'}
                      </span>
                    </td>
                    <td className="px-6 py-3 font-medium text-[#e4e6eb]">{live.app || '-'}</td>
                    <td className="max-w-[360px] truncate px-6 py-3 text-[#a0a5b2]">{live.title || '(sin titulo)'}</td>
                    <td className="px-6 py-3 font-mono text-[#00d9ff]">{live.ago_sec}s</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </AppShell>
  );
}
