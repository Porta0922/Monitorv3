import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { apiClient } from '../api/client';
import { SimpleBarChart } from '../components/metrics/SimpleBarChart';
import { MaintenancePanel } from '../components/metrics/MaintenancePanel';
import { DailyMetricsPanel } from '../components/metrics/DailyMetricsPanel';
import { Icon } from '../components/Icon';
import type { Device } from '../types';
type Overview = Awaited<ReturnType<typeof apiClient.getOverview>>;
interface Hourly { hour: number; active_seconds: number; idle_seconds: number }
const today = () => { const now = new Date(); return `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`; };
export function MetricsPage() {
  const [selectedDeviceId, setDeviceId] = useState('');
  const [date, setDate] = useState(today);
  const [revision, setRevision] = useState(0);
  const [summaryResult, setSummaryResult] = useState<{ revision: number; overview: Overview | null; devices: Device[]; error: string } | null>(null);
  const summary = summaryResult?.revision === revision ? summaryResult : null;
  const overview = summary?.overview;
  const devices = summary?.devices ?? [];
  const deviceId = devices.some(d => d.device_id === selectedDeviceId) ? selectedDeviceId : devices[0]?.device_id ?? '';
  const error = summary?.error ?? '';
  const loading = summary === null;
  const hourlyKey = JSON.stringify([deviceId, date, revision]);
  const [hourlyResult, setHourlyResult] = useState<{ key: string; data: Hourly[] | null; error: string } | null>(null);
  const currentHourly = hourlyResult?.key === hourlyKey ? hourlyResult : null;
  const hourly = currentHourly?.data ?? null;
  const hourlyError = currentHourly?.error ?? '';
  useEffect(() => {
    let alive = true;
    Promise.allSettled([apiClient.getOverview(), apiClient.getDevices()] as const).then(([summary, fleet]) => {
      if (!alive) return;
      const failures: string[] = [];
      if (summary.status === 'rejected') failures.push('resumen');
      if (fleet.status === 'rejected') failures.push('equipos');
      setSummaryResult({ revision, overview: summary.status === 'fulfilled' ? summary.value : null,
        devices: fleet.status === 'fulfilled' ? fleet.value : [],
        error: failures.length ? `No se pudo consultar: ${failures.join(', ')}.` : '' });
    });
    return () => { alive = false; };
  }, [revision]);
  useEffect(() => {
    let alive = true;
    if (!deviceId) return;
    apiClient.getHourly(deviceId, date)
      .then(result => { if (alive) setHourlyResult({ key: hourlyKey, data: result as Hourly[], error: '' }); })
      .catch(() => { if (alive) setHourlyResult({ key: hourlyKey, data: null, error: 'No se pudo consultar la actividad horaria.' }); });
    return () => { alive = false; };
  }, [deviceId, date, hourlyKey]);
  return <AppShell currentPage="metrics" title="Métricas e históricos" subtitle="Consulta las muestras recientes y los resúmenes conservados de cada equipo." actions={<button className="button" disabled={loading} onClick={() => setRevision(v => v + 1)}><Icon name="refresh" />Actualizar</button>}>
    {error && <p className="notice notice-warning" role="alert">{error}</p>}
    <section className="fleet-kpis" aria-label="Resumen global de hoy">{[
      ['Equipos con actividad hoy', overview?.devices_today], ['Pulsaciones registradas', overview?.keys_today], ['Clics registrados', overview?.mouse_clicks_today], ['Movimientos registrados', overview?.mouse_moves_today],
    ].map(([label, value]) => <article className="fleet-kpi" key={String(label)}><div className="kpi-heading">{label}</div><strong className="kpi-value">{typeof value === 'number' ? value.toLocaleString() : '—'}</strong><p>Resumen global del día · según datos recibidos</p></article>)}</section>
    <section className="panel"><div className="panel-header"><div><p className="eyebrow">Explorar por equipo</p><h2>Actividad registrada</h2></div>{deviceId && <Link className="text-link" to={`/devices/${deviceId}`}>Abrir equipo <Icon name="arrow" /></Link>}</div><div className="fleet-toolbar"><label className="metric-selector">Equipo<select value={deviceId} onChange={e => setDeviceId(e.target.value)} disabled={!devices.length}>{!devices.length && <option value="">Sin equipos disponibles</option>}{devices.map(d => <option key={d.device_id} value={d.device_id}>{d.nickname || d.hostname}</option>)}</select></label><label className="metric-selector">Día<input type="date" value={date} max={today()} onChange={e => { if (e.target.value) setDate(e.target.value); }} /></label></div><p className="panel-note">Detalle horario en la zona local del navegador. Las barras muestran minutos registrados; ausencia de actividad no confirma que el recolector esté disponible.</p>
      {!deviceId ? <div className="empty-state">Selecciona un equipo para consultar su actividad.</div> : hourlyError ? <p className="notice notice-error" role="alert">{hourlyError}</p> : hourly === null ? <div className="empty-state" role="status">Cargando actividad…</div> : hourly.length === 0 ? <div className="empty-state"><h3>Sin muestras para este día</h3><p>Selecciona otra fecha o consulta los resúmenes diarios.</p></div> : <div className="metric-charts">{[['Actividad', 'active_seconds', 'var(--palette-508967)'], ['Inactividad', 'idle_seconds', 'var(--palette-b19558)']].map(([title, key, color]) => <div key={key}><h3>{title} · minutos por hora</h3><SimpleBarChart data={hourly.map(row => ({ ...row, [key]: Math.round(row[key as 'active_seconds' | 'idle_seconds'] / 60 * 10) / 10 }))} dataKey={key} label={`${title} en minutos`} color={color} /></div>)}</div>}
    </section>
    {deviceId && <DailyMetricsPanel key={`${deviceId}-${revision}`} deviceId={deviceId} endDate={date} />}
    {apiClient.getRole()==='admin'&&<MaintenancePanel />}
  </AppShell>;
}
