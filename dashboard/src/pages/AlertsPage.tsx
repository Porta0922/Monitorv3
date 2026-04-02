import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import type { SecurityAlert } from '../types';

export function AlertsPage() {
  const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadAlerts();
  }, []);

  const loadAlerts = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getAlerts(undefined, false);
      setAlerts(data);
    } catch (err) {
      console.error('Error loading alerts:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleResolveAlert = async (alertId: number) => {
    try {
      await apiClient.resolveAlert(alertId);
      loadAlerts();
    } catch (err) {
      console.error('Error resolving alert:', err);
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'CRITICAL':
        return '#c33';
      case 'HIGH':
        return '#f80';
      case 'MEDIUM':
        return '#fc0';
      case 'LOW':
        return '#0066cc';
      default:
        return '#666';
    }
  };

  const getAlertIcon = (alertType: string) => {
    switch (alertType) {
      case 'PROCESS_TERMINATION_ATTEMPTED':
        return '⚠️ ';
      case 'HASH_MISMATCH':
        return '🔒 ';
      case 'UNAUTHORIZED_ACCESS':
        return '🚫 ';
      default:
        return '🔔 ';
    }
  };

  // Filter for critical process termination alerts
  const criticalAlerts = alerts.filter(a => a.alert_type === 'PROCESS_TERMINATION_ATTEMPTED');


  return (
    <AppShell
      currentPage="alerts"
      title="Alertas de Seguridad"
      subtitle="Eventos criticos, hash mismatch y alertas operativas"
      actions={
        <button
          onClick={loadAlerts}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      {criticalAlerts.length > 0 && (
        <section className="rounded-xl border border-red-500/40 bg-red-500/10 p-5 shadow-xl">
          <h3 className="text-lg font-semibold text-red-300">CRITICAL: Process Termination Attempts</h3>
          <p className="mt-1 text-sm text-red-200/90">
            Se detectaron {criticalAlerts.length} intentos de terminacion de agente.
          </p>
        </section>
      )}

      {isLoading ? (
        <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] px-6 py-10 text-center text-[#a0a5b2] shadow-2xl">
          Cargando alertas...
        </section>
      ) : alerts.length === 0 ? (
        <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] px-6 py-10 text-center text-[#00ff88] shadow-2xl">
          Sin alertas activas.
        </section>
      ) : (
        <section className="grid gap-4">
          {alerts.map((alert) => (
            <article
              key={alert.id}
              className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-xl"
              style={{ borderLeftColor: getSeverityColor(alert.severity), borderLeftWidth: 4 }}
            >
              <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h3 className="text-base font-semibold text-[#e4e6eb]">
                    {getAlertIcon(alert.alert_type)} {alert.alert_type}
                  </h3>
                  <p className="mt-1 text-xs text-[#a0a5b2]">
                    {alert.app_name ? `Application: ${alert.app_name}` : 'System Alert'}
                  </p>
                </div>
                <span
                  className="rounded-full border px-3 py-1 text-xs font-semibold"
                  style={{ color: getSeverityColor(alert.severity), borderColor: `${getSeverityColor(alert.severity)}88` }}
                >
                  {alert.severity}
                </span>
              </div>

              <p className="mb-3 text-sm text-[#a0a5b2]">{alert.description}</p>

              <div className="mb-4 space-y-1 text-xs text-[#717579]">
                <p>Created: {new Date(alert.created_at).toLocaleString()}</p>
                {alert.exe_hash && <p className="font-mono">Hash: {alert.exe_hash.slice(0, 32)}...</p>}
              </div>

              <button
                onClick={() => handleResolveAlert(alert.id)}
                className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-xs font-semibold text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
              >
                Marcar como resuelta
              </button>
            </article>
          ))}
        </section>
      )}
    </AppShell>
  );
}
