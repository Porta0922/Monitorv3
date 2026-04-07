import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { USBEvent } from '../types';

export function USBPage() {
  const [events, setEvents] = useState<USBEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadUSBHistory();
  }, []);

  const loadUSBHistory = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getUsbHistory(undefined, 1000);
      setEvents(data);
    } catch (err) {
      console.error('Error loading USB history:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="usb"
      title="Eventos USB"
      subtitle="Historial de conexiones y desconexiones de dispositivos"
      actions={
        <button
          onClick={loadUSBHistory}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      <section className="overflow-hidden rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl">
        <div className="border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <h2 className="text-lg font-semibold text-[#e4e6eb]">Eventos recientes ({events.length})</h2>
        </div>

        {isLoading ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Cargando eventos USB...</div>
        ) : events.length === 0 ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">No hay eventos USB registrados.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Fecha/Hora</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Dispositivo</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Nombre</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Serial</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Accion</th>
                </tr>
              </thead>
              <tbody>
                {events.slice(0, 120).map((event, idx) => (
                  <tr key={`${event.device_id}-${event.timestamp}-${idx}`} className="border-b border-[#1e2339] hover:bg-[#131829]">
                    <td className="px-6 py-3 text-[#a0a5b2]">{new Date(event.timestamp).toLocaleString()}</td>
                    <td className="px-6 py-3 font-mono text-xs text-[#a0a5b2]">{event.device_id.slice(0, 8)}...</td>
                    <td className="px-6 py-3 font-medium text-[#e4e6eb]">{event.device_name}</td>
                    <td className="px-6 py-3 font-mono text-xs text-[#a0a5b2]">{event.serial_number}</td>
                    <td className="px-6 py-3">
                      <span
                        className={`rounded-full border px-3 py-1 text-xs font-semibold ${
                          event.action === 'IN'
                            ? 'border-[#00ff88]/50 bg-[#00ff88]/10 text-[#00ff88]'
                            : 'border-red-400/50 bg-red-500/10 text-red-400'
                        }`}
                      >
                        {event.action === 'IN' ? 'Conectado' : 'Desconectado'}
                      </span>
                    </td>
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
