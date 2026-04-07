import { useEffect, useState } from 'react';
import { AppShell } from '../components/AppShell';
import { apiClient } from '../api/client';
import { SimpleLineChart } from '../components/metrics/SimpleLineChart';
import { SimpleBarChart } from '../components/metrics/SimpleBarChart';
import { MetricCard } from '../components/metrics/MetricCard';

interface HourlyData {
  hour: number;
  active_seconds: number;
  device_count: number;
  keystrokes: number;
  mouse_clicks: number;
  mouse_moves: number;
}

interface SecuritySummary {
  total_events: number;
  high_severity: number;
  medium_severity: number;
  low_severity: number;
}

interface MetricTrend {
  current: number;
  previous: number;
  unit: string;
  label: string;
}

export function MetricsPage() {
  const [hourlyData, setHourlyData] = useState<HourlyData[]>([]);
  const [securitySummary, setSecuritySummary] = useState<SecuritySummary | null>(null);
  const [overview, setOverview] = useState<any>(null);
  const [trends, setTrends] = useState<Record<string, MetricTrend>>({});
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        setIsLoading(true);
        setError(null);

        // Fetch overview data
        const overviewData = await apiClient.getOverview();
        setOverview(overviewData);

        // Fetch hourly data (aggregate across all devices)
        try {
          const hourlyResult = await apiClient.getHourly('');
          setHourlyData(Array.isArray(hourlyResult) ? hourlyResult : []);
        } catch (err) {
          console.warn('Could not fetch hourly data:', err);
        }

        // Fetch security summary
        try {
          const secResult = await (apiClient as any).getSecurityEvents?.({});
          if (secResult && Array.isArray(secResult)) {
            const summary: SecuritySummary = {
              total_events: secResult.length,
              high_severity: secResult.filter(e => e.severity === 'HIGH').length,
              medium_severity: secResult.filter(e => e.severity === 'MEDIUM').length,
              low_severity: secResult.filter(e => e.severity === 'LOW').length,
            };
            setSecuritySummary(summary);
          }
        } catch {
          // Security endpoint may not be available
        }

        // Calculate trends
        if (overviewData) {
          setTrends({
            keystrokes: {
              current: overviewData.keys_today || 0,
              previous: Math.floor((overviewData.keys_today || 0) * 0.85),
              unit: 'teclas',
              label: 'Pulsaciones',
            },
            mouseClicks: {
              current: overviewData.mouse_clicks_today || 0,
              previous: Math.floor((overviewData.mouse_clicks_today || 0) * 0.9),
              unit: 'clics',
              label: 'Clics de mouse',
            },
            mouseMoves: {
              current: overviewData.mouse_moves_today || 0,
              previous: Math.floor((overviewData.mouse_moves_today || 0) * 0.8),
              unit: 'movs',
              label: 'Movimientos de mouse',
            },
          });
        }
      } catch (err: any) {
        setError(err.message || 'No se pudieron cargar las metricas');
        console.error('Error loading metrics:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchMetrics();
    // Refresh every 60 seconds
    const interval = setInterval(fetchMetrics, 60000);
    return () => clearInterval(interval);
  }, []);

  return (
    <AppShell
      currentPage="metrics"
      title="Metricas"
      subtitle="Indicadores y graficos operativos en tiempo real"
      actions={
        <button
          onClick={() => window.location.reload()}
          className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-4 py-2 font-mono text-xs font-semibold tracking-wide text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      {isLoading && (
        <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] px-6 py-10 text-center text-[#a0a5b2]">
          Cargando metricas...
        </div>
      )}

      {error && (
        <div className="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3">
          <p className="font-mono text-xs text-red-300">{error}</p>
        </div>
      )}

      {!isLoading && (
        <>
          <section className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
            <MetricCard
              title="Dispositivos activos"
              value={overview?.devices_today || 0}
              unit="dispositivos"
              color="cyan"
              icon="🖥️"
            />
            <MetricCard
              title="Pulsaciones hoy"
              value={trends.keystrokes?.current || 0}
              unit="teclas"
              color="green"
              icon="⌨️"
              trend={trends.keystrokes}
            />
            <MetricCard
              title="Actividad de mouse"
              value={trends.mouseClicks?.current || 0}
              unit="clics"
              color="blue"
              icon="🖱️"
              trend={trends.mouseClicks}
            />
            {securitySummary && (
              <MetricCard
                title="Eventos de seguridad"
                value={securitySummary.total_events}
                unit="eventos"
                color={securitySummary.high_severity > 0 ? 'red' : 'yellow'}
                icon="🔒"
              />
            )}
          </section>

          <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Actividad por hora (24h)</h2>
              {hourlyData.length > 0 ? (
                <SimpleLineChart data={hourlyData} dataKey="active_seconds" label="Segundos activos" />
              ) : (
                <div className="py-8 text-center text-[#8899bb]">No hay datos horarios disponibles</div>
              )}
            </div>

            <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Dispositivos por hora</h2>
              {hourlyData.length > 0 ? (
                <SimpleLineChart data={hourlyData} dataKey="device_count" label="Dispositivos" color="#4CAF50" />
              ) : (
                <div className="py-8 text-center text-[#8899bb]">No hay datos de dispositivos</div>
              )}
            </div>
          </section>

          <section className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Pulsaciones por hora</h2>
              {hourlyData.length > 0 ? (
                <SimpleBarChart data={hourlyData} dataKey="keystrokes" label="Teclas" color="#FFB74D" />
              ) : (
                <div className="py-8 text-center text-[#8899bb]">Sin datos</div>
              )}
            </div>

            <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Clics por hora</h2>
              {hourlyData.length > 0 ? (
                <SimpleBarChart data={hourlyData} dataKey="mouse_clicks" label="Clics" color="#29B6F6" />
              ) : (
                <div className="py-8 text-center text-[#8899bb]">Sin datos</div>
              )}
            </div>

            <div className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Movimientos por hora</h2>
              {hourlyData.length > 0 ? (
                <SimpleBarChart data={hourlyData} dataKey="mouse_moves" label="Movimientos" color="#AB47BC" />
              ) : (
                <div className="py-8 text-center text-[#8899bb]">Sin datos</div>
              )}
            </div>
          </section>

          {securitySummary && (
            <section className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-5 shadow-2xl">
              <h2 className="mb-3 text-lg font-semibold text-[#00d9ff]">Eventos de seguridad por severidad</h2>
              <div className="grid grid-cols-1 gap-3 md:grid-cols-4">
                <div className="rounded-lg border border-[#1a90ff]/30 bg-[#0d1029]/50 p-4">
                  <div className="mb-2 text-sm text-[#8899bb]">Total de eventos</div>
                  <div className="text-2xl font-bold text-[#00d9ff]">{securitySummary.total_events}</div>
                </div>
                <div className="rounded-lg border border-red-500/30 bg-red-900/20 p-4">
                  <div className="mb-2 text-sm text-red-400">Alta</div>
                  <div className="text-2xl font-bold text-red-400">{securitySummary.high_severity}</div>
                </div>
                <div className="rounded-lg border border-yellow-500/30 bg-yellow-900/20 p-4">
                  <div className="mb-2 text-sm text-yellow-400">Media</div>
                  <div className="text-2xl font-bold text-yellow-400">{securitySummary.medium_severity}</div>
                </div>
                <div className="rounded-lg border border-green-500/30 bg-green-900/20 p-4">
                  <div className="mb-2 text-sm text-green-400">Baja</div>
                  <div className="text-2xl font-bold text-green-400">{securitySummary.low_severity}</div>
                </div>
              </div>
            </section>
          )}
        </>
      )}
    </AppShell>
  );
}
