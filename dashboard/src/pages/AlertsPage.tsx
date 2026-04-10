import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { SecurityAlert, DeviceResourcePeak } from '../types';
import { useNavigate } from 'react-router-dom';

interface DeviceMeta {
  nodeName: string;
  macAddress: string;
}

export function AlertsPage() {
  const navigate = useNavigate();
  const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
  const [resourcePeaks, setResourcePeaks] = useState<DeviceResourcePeak[]>([]);
  const [deviceNames, setDeviceNames] = useState<Map<string, string>>(new Map());
  const [deviceMeta, setDeviceMeta] = useState<Map<string, DeviceMeta>>(new Map());
  const [resolvingIds, setResolvingIds] = useState<Set<number>>(new Set());
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    load();
  }, []);

  const load = async () => {
    try {
      setIsLoading(true);
      const [alertsData, peaksData, devicesData] = await Promise.all([
        apiClient.getAlerts(undefined, false).catch(() => [] as SecurityAlert[]),
        apiClient.getResourcePeaks(50).catch(() => [] as DeviceResourcePeak[]),
        apiClient.getDevices().catch(() => []),
      ]);
      setAlerts(alertsData);
      setResourcePeaks(peaksData as DeviceResourcePeak[]);
      setDeviceNames(new Map(devicesData.map((d) => [d.device_id, d.nickname || d.hostname])));
      setDeviceMeta(
        new Map(
          devicesData.map((d) => [
            d.device_id,
            {
              nodeName: d.nickname || d.hostname || d.device_id.slice(0, 8),
              macAddress: d.mac_address || 'N/A',
            },
          ])
        )
      );
    } catch (err) {
      console.error('Error loading alerts:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleResolveAlert = async (alertId: number) => {
    if (resolvingIds.has(alertId)) return;

    try {
      setResolvingIds((prev) => new Set(prev).add(alertId));
      await apiClient.resolveAlert(alertId);
      setAlerts((prev) => prev.filter((alert) => alert.id !== alertId));
    } catch (err) {
      console.error('Error resolving alert:', err);
    } finally {
      setResolvingIds((prev) => {
        const next = new Set(prev);
        next.delete(alertId);
        return next;
      });
    }
  };

  const severityBadge = (severity: string) => {
    const map: Record<string, string> = {
      CRITICAL: 'border-red-500/50 bg-red-500/10 text-red-300',
      HIGH:     'border-[#ff9f1a]/50 bg-[#ff9f1a]/10 text-[#ff9f1a]',
      MEDIUM:   'border-[#ffd54a]/50 bg-[#ffd54a]/10 text-[#ffd54a]',
      LOW:      'border-[#00d9ff]/50 bg-[#00d9ff]/10 text-[#00d9ff]',
    };
    return `inline-flex rounded-full border px-2.5 py-0.5 font-mono text-[10px] ${map[severity] ?? 'border-[#223462] text-[#8ea0cf]'}`;
  };

  // Resource peaks above threshold (CPU >80% or RAM >88%)
  const hotPeaks = resourcePeaks.filter(
    (p) => (p.peak_cpu_percent || 0) > 80 || (p.peak_memory_percent || 0) > 88
  );

  return (
    <AppShell
      currentPage="alerts"
      title="Alertas de Seguridad"
      subtitle="Seguridad y picos de recursos por nodo"
      noScroll
      actions={
        <button
          onClick={load}
          className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-3 py-1.5 font-mono text-[10px] text-[#00d9ff] hover:border-[#00d9ff]"
        >
          Actualizar
        </button>
      }
    >
      <div className="flex h-[calc(100vh-190px)] flex-col gap-4 overflow-hidden">

        {/* Resource peaks strip */}
        {hotPeaks.length > 0 && (
          <section className="shrink-0 rounded-2xl border border-[#ff5f7a]/30 bg-[linear-gradient(160deg,#1f0a14,#0b1329)] p-4 shadow-[0_10px_22px_rgba(0,0,0,0.3)]">
            <div className="mb-3 flex items-center gap-3">
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#ff8ea0]">Picos de recursos — hoy</p>
              <span className="rounded-full border border-[#ff5f7a]/40 bg-[#ff5f7a]/10 px-2.5 py-0.5 font-mono text-[10px] text-[#ff8ea0]">
                {hotPeaks.length} nodos
              </span>
            </div>
            <div className="flex flex-wrap gap-2">
              {hotPeaks.map((peak) => (
                <button
                  key={peak.device_id}
                  onClick={() => navigate(`/devices/${peak.device_id}`)}
                  className="flex flex-col gap-1.5 rounded-xl border border-[#ff5f7a]/30 bg-[#0a122a] px-3 py-2 text-left hover:border-[#ff5f7a]/60"
                >
                  <p className="font-mono text-[11px] text-[#dce6ff]">
                    {deviceNames.get(peak.device_id) || peak.device_id.slice(0, 8)}
                  </p>
                  <div className="flex gap-3">
                    <span className="flex items-center gap-1">
                      <span className="h-1.5 w-1.5 rounded-full bg-[#ff5f7a]" />
                      <span className="font-mono text-[10px] text-[#ff8ea0]">CPU {Math.round(peak.peak_cpu_percent || 0)}%</span>
                    </span>
                    <span className="flex items-center gap-1">
                      <span className="h-1.5 w-1.5 rounded-full bg-[#00d9ff]" />
                      <span className="font-mono text-[10px] text-[#7deeff]">RAM {Math.round(peak.peak_memory_percent || 0)}%</span>
                    </span>
                  </div>
                  {peak.top_process_name && (
                    <p className="font-mono text-[9px] text-[#5a6a90]">▸ {peak.top_process_name}</p>
                  )}
                </button>
              ))}
            </div>
          </section>
        )}

        {/* Security alerts table */}
        <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-[#1a2748] bg-[linear-gradient(165deg,#0f1d43,#0b1329)] shadow-[0_14px_30px_rgba(0,0,0,0.35)]">
          <div className="flex shrink-0 items-center justify-between border-b border-[#20315a] px-5 py-3">
            <div className="flex items-center gap-3">
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#8ea0cf]">Alertas de seguridad</p>
              <span className="rounded-full border border-[#ff5f7a]/40 bg-[#ff5f7a]/10 px-2.5 py-0.5 font-mono text-[10px] text-[#ff8ea0]">
                {alerts.length}
              </span>
            </div>
          </div>

          <div className="flex-1 overflow-auto">
            {isLoading ? (
              <p className="py-10 text-center font-mono text-[11px] text-[#5a6a90]">Cargando alertas...</p>
            ) : alerts.length === 0 ? (
              <p className="py-10 text-center font-mono text-[11px] text-[#00ff88]">Sin alertas activas.</p>
            ) : (
              <table className="w-full border-collapse">
                <thead>
                  <tr className="sticky top-0 z-10 border-b border-[#20315a] bg-[#0a122a]">
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Severidad</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Tipo</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Nodo</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">MAC</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Descripcion</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">App</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Fecha</th>
                    <th className="px-5 py-2.5 text-left font-mono text-[10px] uppercase tracking-[0.18em] text-[#7c90c1]">Accion</th>
                  </tr>
                </thead>
                <tbody>
                  {alerts.map((alert) => (
                    <tr key={alert.id} className="border-b border-[#1a2748] hover:bg-[#0f1c3a]">
                      <td className="px-5 py-2.5">
                        <span className={severityBadge(alert.severity)}>{alert.severity}</span>
                      </td>
                      <td className="px-5 py-2.5 font-mono text-[10px] text-[#8ea0cf]">{alert.alert_type}</td>
                      <td className="px-5 py-2.5 font-mono text-[11px] text-[#dce6ff]">
                        {deviceMeta.get(alert.device_id)?.nodeName || deviceNames.get(alert.device_id) || alert.device_id.slice(0, 8)}
                      </td>
                      <td className="px-5 py-2.5 font-mono text-[10px] text-[#8ea0cf]">
                        {deviceMeta.get(alert.device_id)?.macAddress || 'N/A'}
                      </td>
                      <td className="max-w-[320px] truncate px-5 py-2.5 font-mono text-[11px] text-[#dce6ff]">{alert.description}</td>
                      <td className="px-5 py-2.5 font-mono text-[10px] text-[#8ea0cf]">{alert.app_name || '—'}</td>
                      <td className="px-5 py-2.5 font-mono text-[10px] text-[#7c90c1]">
                        {new Date(alert.created_at).toLocaleString()}
                      </td>
                      <td className="px-5 py-2.5">
                        <button
                          onClick={() => handleResolveAlert(alert.id)}
                          disabled={resolvingIds.has(alert.id)}
                          className="rounded-full border border-[#00ff88]/40 bg-[#00ff88]/10 px-3 py-1 font-mono text-[10px] text-[#00ff88] hover:border-[#00ff88] disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          {resolvingIds.has(alert.id) ? 'Resolviendo...' : 'Resolver'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </section>
      </div>
    </AppShell>
  );
}
