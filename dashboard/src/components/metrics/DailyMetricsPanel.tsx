import { useEffect, useState } from 'react';
import { apiClient } from '../../api/client';
import type { DailyMetricsResponse } from '../../types';

const sourceLabel = { detail: 'Muestras', summary: 'Resumen diario', mixed: 'Resumen + muestras' };
const percent = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`;
const hours = (value: number | null) => value === null ? '—' : `${(value / 3600).toFixed(2)} h`;

export function DailyMetricsPanel({ deviceId, endDate }: { deviceId: string; endDate: string }) {
  const [range, setRange] = useState(30);
  const [retry, setRetry] = useState(0);
  const requestKey = JSON.stringify([deviceId, endDate, range, retry]);
  const [result, setResult] = useState<{ key: string; data: DailyMetricsResponse | null; error: string } | null>(null);
  const current = result?.key === requestKey ? result : null;
  const loading = current === null;
  const data = current?.data;
  const error = current?.error ?? '';

  useEffect(() => {
    const controller = new AbortController();
    // A local calendar day can be ahead of the current UTC day.
    const effectiveEnd = endDate > new Date().toISOString().slice(0, 10) ? new Date().toISOString().slice(0, 10) : endDate;
    const start = new Date(`${effectiveEnd}T00:00:00Z`);
    start.setUTCDate(start.getUTCDate() - range + 1);
    apiClient.getDailyMetrics(deviceId, start.toISOString().slice(0, 10), effectiveEnd, controller.signal)
      .then(data => { if (!controller.signal.aborted) setResult({ key: requestKey, data, error: '' }); })
      .catch(() => { if (!controller.signal.aborted) setResult({ key: requestKey, data: null, error: 'No se pudo cargar el historial diario.' }); });
    return () => controller.abort();
  }, [deviceId, endDate, range, requestKey]);

  return (
    <section className="panel p-6" aria-labelledby="daily-metrics-title">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 id="daily-metrics-title" className="text-sm font-semibold text-[var(--palette-53685c)]">Historial diario de recursos y actividad</h2>
          <p className="mt-1 text-xs text-[var(--palette-53685c)]">Días UTC · CPU/RAM: promedio por muestra · Actividad: duración acumulada</p>
        </div>
        <label className="flex items-center gap-2 text-xs text-[var(--palette-53685c)]">
          Período
          <select aria-label="Período del historial diario" value={range} onChange={event => setRange(Number(event.target.value))}
            className="rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2 text-[var(--palette-53685c)]">
            {[7, 30, 90, 365].map(days => <option key={days} value={days}>{days} días</option>)}
          </select>
        </label>
      </div>
      <p className="mt-3 text-xs text-[var(--palette-53685c)]">Detalle reciente y resúmenes conservados se combinan una sola vez por día. “—” indica que no hay datos de esa métrica. La calidad cuenta muestras, no horas cubiertas: los registros heredados quedan sin confirmar; un cero no demuestra disponibilidad.</p>
      {loading && <p role="status" className="py-6 text-sm text-[var(--palette-53685c)]">Cargando historial diario…</p>}
      {error && <div role="alert" className="mt-4 flex items-center gap-3 text-sm text-red-700">
        <span>{error}</span><button className="rounded border border-red-300/40 px-3 py-1" onClick={() => setRetry(value => value + 1)}>Reintentar</button>
      </div>}
      {!loading && !error && data && (data.days.length === 0
        ? <p className="py-6 text-sm text-[var(--palette-53685c)]">Sin datos conservados entre {data.from} y {data.to} (UTC).</p>
        : <div className="mt-4 overflow-x-auto">
          <table className="w-full text-left text-xs text-[var(--palette-53685c)]">
            <caption className="sr-only">Historial entre {data.from} y {data.to}, fechas en UTC</caption>
            <thead className="border-b border-[var(--palette-dfe7e3)] text-[var(--palette-53685c)]"><tr>
              {['Día UTC', 'Origen', 'Muestras CPU/RAM', 'CPU promedio / pico', 'RAM promedio / pico', 'Actividad', 'Inactividad', 'Calidad de entrada'].map(label =>
                <th key={label} scope="col" className="whitespace-nowrap px-3 py-3 font-medium">{label}</th>)}
            </tr></thead>
            <tbody>{data.days.map(day => <tr key={day.day} className="border-b border-[var(--palette-dfe7e3)] last:border-0">
              <th scope="row" className="whitespace-nowrap px-3 py-3 font-sans font-normal">{day.day}</th>
              <td className="px-3 py-3"><span className="rounded bg-[var(--palette-f5f7f6)] px-2 py-1">{sourceLabel[day.source]}</span></td>
              <td className="px-3 py-3 font-sans">{day.resource_samples.toLocaleString()}</td>
              <td className="whitespace-nowrap px-3 py-3 font-sans">{percent(day.cpu_average)} / {percent(day.cpu_peak)}</td>
              <td className="whitespace-nowrap px-3 py-3 font-sans">{percent(day.memory_average)} / {percent(day.memory_peak)}</td>
              <td className="whitespace-nowrap px-3 py-3 font-sans">{hours(day.active_seconds)}</td>
              <td className="whitespace-nowrap px-3 py-3 font-sans">{hours(day.idle_seconds)}</td>
              <td className="px-3 py-3">{day.input_available_samples} disponibles · {day.input_unavailable_samples} no disponibles · {day.input_unknown_samples} sin confirmar</td>
            </tr>)}</tbody>
          </table>
        </div>)}
    </section>
  );
}
