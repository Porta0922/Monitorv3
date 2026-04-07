import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { AppInfo } from '../types';

export function InventoryPage() {
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSoftwareInventory();
  }, []);

  const loadSoftwareInventory = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getApps();
      setApps(data);
    } catch (err) {
      console.error('Error loading software inventory:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <AppShell
      currentPage="inventory"
      title="Inventario de Software"
      subtitle="Aplicaciones instaladas y validacion de binarios"
      actions={
        <button
          onClick={loadSoftwareInventory}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      <section className="overflow-hidden rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl">
        <div className="border-b border-[#1e2339] bg-[#0a0e27] px-6 py-4">
          <h2 className="text-lg font-semibold text-[#e4e6eb]">Aplicaciones detectadas ({apps.length})</h2>
        </div>

        {isLoading ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Cargando inventario...</div>
        ) : apps.length === 0 ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">No hay inventario disponible.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Aplicacion</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Version</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Estado</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Hash SHA-256</th>
                </tr>
              </thead>
              <tbody>
                {apps.slice(0, 120).map((app, idx) => (
                  <tr key={`${app.app_name}-${idx}`} className="border-b border-[#1e2339] hover:bg-[#131829]">
                    <td className="px-6 py-3 font-medium text-[#e4e6eb]">{app.app_name}</td>
                    <td className="px-6 py-3 text-[#a0a5b2]">{app.version || 'Desconocida'}</td>
                    <td className="px-6 py-3">
                      <span
                        className={`rounded-full border px-3 py-1 text-xs font-semibold ${
                          app.verified
                            ? 'border-[#00ff88]/50 bg-[#00ff88]/10 text-[#00ff88]'
                            : 'border-red-400/50 bg-red-500/10 text-red-400'
                        }`}
                      >
                        {app.verified ? 'Verificada' : 'No verificada'}
                      </span>
                    </td>
                    <td className="max-w-[380px] truncate px-6 py-3 font-mono text-xs text-[#a0a5b2]">{app.exe_hash}</td>
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
