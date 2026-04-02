import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { ActivityLog } from '../types';

export function ActivityPage() {
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadActivityLogs();
  }, []);

  const loadActivityLogs = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getActivityLogs(undefined, 1000);
      setLogs(data);
    } catch (err) {
      console.error('Error loading activity logs:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="activity"
      title="Registro de Actividad"
      subtitle="Eventos recientes de aplicaciones y ventanas en tiempo real"
      actions={
        <button
          onClick={loadActivityLogs}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl overflow-hidden">
        <div className="border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <h2 className="text-lg font-semibold text-[#e4e6eb]">Ultimos eventos ({logs.length})</h2>
        </div>

        {isLoading ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Cargando actividad...</div>
        ) : logs.length === 0 ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">No hay actividad registrada aun.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Timestamp</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Device</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Aplicacion</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Ventana</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Duracion</th>
                </tr>
              </thead>
              <tbody>
                {logs.slice(0, 120).map((log, idx) => (
                  <tr key={`${log.device_id}-${log.timestamp}-${idx}`} className="border-b border-[#1e2339] hover:bg-[#131829]">
                    <td className="px-6 py-3 text-[#a0a5b2]">{new Date(log.timestamp).toLocaleString()}</td>
                    <td className="px-6 py-3 font-mono text-xs text-[#a0a5b2]">{log.device_id.slice(0, 8)}...</td>
                    <td className="px-6 py-3 font-medium text-[#e4e6eb]">{log.app_name}</td>
                    <td className="max-w-[360px] truncate px-6 py-3 text-[#a0a5b2]">{log.window_title}</td>
                    <td className="px-6 py-3 font-mono text-[#00ff88]">{log.duration_seconds}s</td>
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
