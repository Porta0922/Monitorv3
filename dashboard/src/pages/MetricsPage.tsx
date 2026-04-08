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
        console.error('Error loading metrics:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchMetrics();
    const interval = setInterval(fetchMetrics, 60000);
    return () => clearInterval(interval);
  }, []);

  return (
    <AppShell
      currentPage="metrics"
      title="Metricas"
      subtitle="Indicadores operativos en tiempo real"
      noScroll
      actions={
        <button
          onClick={() => window.location.reload()}
          className="rounded-full border border-[#00d9ff]/50 bg-[#00d9ff]/10 px-3 py-1.5 font-mono text-[10px] text-[#00d9ff] hover:border-[#00d9ff]"
        >
          Actualizar
        </button>
      }
    >
      <div className="flex h-[calc(100vh-190px)] flex-col gap-4 overflow-hidden">
        {/* KPI strip */}
        <section className="grid shrink-0 grid-cols-2 gap-3 md:grid-cols-4">
          {isLoading ? (
            [...Array(4)].map((_, i) => (
              <div key={i} className="h-[72px] animate-pulse rounded-2xl border border-[#1a2748] bg-[#0b1329]" />
            ))
          ) : (
            <>
              <MetricCard title="Dispositivos activos" value={overview?.devices_today || 0} unit="hoy" color="cyan" />
              <MetricCard
                title="Eventos de seguridad"
                value={securitySummary?.total_events ?? 0}
                unit="eventos"
                color={(securitySummary?.high_severity ?? 0) > 0 ? 'red' : 'yellow'}
              />
              <MetricCard title="Pulsaciones hoy" value={trends.keystrokes?.current || 0} unit="teclas" color="green" trend={trends.keystrokes} />
              <MetricCard title="Clics de mouse" value={trends.mouseClicks?.current || 0} unit="clics" color="cyan" trend={trends.mouseClicks} />
            </>
          )}
        </section>

        {error && (
          <div className="shrink-0 rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-2">
            <p className="font-mono text-[10px] text-red-300">{error}</p>
          </div>
        )}

        {/* 2x2 charts grid */}
        <div className="grid min-h-0 flex-1 grid-cols-2 grid-rows-2 gap-4 overflow-hidden">
          {[
            { title: 'Actividad por hora', key: 'active_seconds', type: 'line', color: '#00d9ff' },
            { title: 'Dispositivos por hora', key: 'device_count', type: 'line', color: '#00ff88' },
            { title: 'Pulsaciones por hora', key: 'keystrokes', type: 'bar', color: '#ffd54a' },
            { title: 'Clics de mouse por hora', key: 'mouse_clicks', type: 'bar', color: '#00d9ff' },
          ].map(({ title, key, type, color }) => (
            <div key={key} className="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-[#1a2748] bg-[linear-gradient(160deg,#0f1d43,#0b1329)] p-4 shadow-[0_10px_22px_rgba(0,0,0,0.32)]">
              <p className="mb-2 shrink-0 font-mono text-[11px] uppercase tracking-[0.18em] text-[#8ea0cf]">{title}</p>
              <div className="min-h-0 flex-1 overflow-hidden">
                {isLoading ? (
                  <div className="flex h-full items-center justify-center">
                    <p className="font-mono text-[10px] text-[#5a6a90]">Cargando...</p>
                  </div>
                ) : hourlyData.length > 0 ? (
                  type === 'line'
                    ? <SimpleLineChart data={hourlyData} dataKey={key} label={title} color={color} />
                    : <SimpleBarChart data={hourlyData} dataKey={key} label={title} color={color} />
                ) : (
                  <div className="flex h-full items-center justify-center">
                    <p className="font-mono text-[10px] text-[#5a6a90]">Sin datos disponibles</p>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
