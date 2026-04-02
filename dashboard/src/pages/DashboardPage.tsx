import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import { OverviewCard } from '../components/OverviewCard';
import { TopAppsTable } from '../components/TopAppsTable';
import { LiveActivityTable } from '../components/LiveActivityTable';
import type { Device } from '../types';

export function DashboardPage() {
  const navigate = useNavigate();
  const [devices, setDevices] = useState<Device[]>([]);
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
      const data = await apiClient.getDevices();
      setDevices(data);
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
      title="Dashboard Enterprise"
      subtitle="Monitoreo en tiempo real consolidado en tarjetas"
      actions={
        <button
          onClick={loadDevices}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar dispositivos
        </button>
      }
    >
      <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-1 shadow-2xl">
        <OverviewCard />
      </section>

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-5">
        <section className="xl:col-span-3 rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-1 shadow-2xl">
          <LiveActivityTable />
        </section>
        <section className="xl:col-span-2 rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-1 shadow-2xl">
          <TopAppsTable />
        </section>
      </div>

      <section className="overflow-hidden rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <div>
            <h2 className="text-xl font-bold text-[#e4e6eb]">Dispositivos Monitoreados</h2>
            <p className="mt-1 text-sm text-[#a0a5b2]">{devices.length} dispositivo(s) registrado(s)</p>
          </div>
        </div>

        {error && (
          <div className="m-6 rounded-lg border border-red-500/30 bg-red-500/10 p-4">
            <p className="text-sm text-red-400">{error}</p>
          </div>
        )}

        <div className="p-6">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <p className="text-[#a0a5b2]">Cargando dispositivos...</p>
            </div>
          ) : devices.length === 0 ? (
            <div className="py-12 text-center">
              <p className="mb-2 text-[#a0a5b2]">Sin dispositivos registrados</p>
              <p className="text-sm text-[#717579]">Los dispositivos apareceran aqui cuando los agentes se conecten.</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 2xl:grid-cols-3">
              {devices.map((device) => (
                <article
                  key={device.device_id}
                  className="rounded-lg border border-[#1e2339] bg-[#0a0e27] p-4 transition-all hover:border-[#00d9ff]/50 hover:shadow-lg hover:shadow-[#00d9ff]/10"
                >
                  <div className="mb-3 flex items-start justify-between gap-3">
                    <div>
                      <h3 className="font-mono font-bold text-[#e4e6eb]">{device.nickname || device.hostname}</h3>
                      <p className="mt-1 font-mono text-xs text-[#717579]">{device.hostname}</p>
                    </div>
                    <span
                      className={`inline-block rounded-full border px-3 py-1 text-xs font-semibold ${
                        device.online
                          ? 'border-[#00ff88]/50 bg-[#00ff88]/20 text-[#00ff88]'
                          : 'border-red-500/50 bg-red-500/20 text-red-400'
                      }`}
                    >
                      {device.online ? 'En linea' : 'Offline'}
                    </span>
                  </div>

                  <div className="mb-4 space-y-2 text-xs text-[#a0a5b2]">
                    <p className="font-mono">MAC: <span className="text-[#717579]">{device.mac_address || 'N/A'}</span></p>
                    <p>Visto: {new Date(device.last_seen).toLocaleString()}</p>
                  </div>

                  <div className="flex gap-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleUpdateNickname(device.device_id, device.nickname);
                      }}
                      className="flex-1 rounded border border-[#00d9ff]/30 bg-[#00d9ff]/15 px-3 py-2 text-xs font-medium text-[#00d9ff] hover:bg-[#00d9ff]/25"
                    >
                      Editar
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        navigate('/activity');
                      }}
                      className="flex-1 rounded border border-[#00ff88]/30 bg-[#00ff88]/15 px-3 py-2 text-xs font-medium text-[#00ff88] hover:bg-[#00ff88]/25"
                    >
                      Ver logs
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </div>
      </section>
    </AppShell>
  );
}
