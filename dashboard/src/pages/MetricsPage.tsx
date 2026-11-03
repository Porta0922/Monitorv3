import {useCallback} from 'react';
import {useRemoteSection} from '../hooks/useRemoteSection';
import {useQueryState} from '../hooks/useQueryState';
import { Link } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { apiClient } from '../api/client';
import { SimpleBarChart } from '../components/metrics/SimpleBarChart';
import { MaintenancePanel } from '../components/metrics/MaintenancePanel';
import { DailyMetricsPanel } from '../components/metrics/DailyMetricsPanel';
import { Icon } from '../components/Icon';


interface Hourly { hour: number; active_seconds: number; idle_seconds: number }
const today = () => new Date().toISOString().slice(0,10);
const loadOverview=()=>apiClient.getOverview();
const loadDevices=()=>apiClient.getDevices();
export function MetricsPage() {
  const [selectedDeviceId,setDeviceId]=useQueryState('device');
  const [requestedDate,setDate]=useQueryState('date',today());
  const date=/^\d{4}-\d{2}-\d{2}$/.test(requestedDate)&&Number.isFinite(Date.parse(requestedDate))&&new Date(requestedDate).toISOString().slice(0,10)===requestedDate&&requestedDate<=today()?requestedDate:today();
  const summary=useRemoteSection(loadOverview), equipment=useRemoteSection(loadDevices);
  const overview=summary.data, devices=equipment.data??[];
  const deviceId=selectedDeviceId||devices[0]?.device_id||'';
  const hourlyState=useRemoteSection(useCallback(async()=>deviceId?await apiClient.getHourly(deviceId,date) as Hourly[]:[],[deviceId,date]));
  const hourly=hourlyState.data;
  const hourlyError=hourlyState.error?'No se pudo actualizar la actividad horaria. Los valores anteriores pueden estar desactualizados.':'';
  const loading=summary.loading||equipment.loading||hourlyState.loading;
  const refresh=()=>{summary.refresh();equipment.refresh();hourlyState.refresh();};
  return <AppShell currentPage="metrics" title="Métricas e históricos" subtitle="Consulta las muestras recientes y los resúmenes conservados de cada equipo." actions={<button className="button" disabled={loading} onClick={refresh}><Icon name="refresh" />Actualizar</button>}>
    {summary.error&&<p className="notice notice-warning" role="alert">No se pudo actualizar el resumen. Los valores anteriores pueden estar desactualizados. <button className="button" onClick={summary.refresh}>Reintentar resumen</button></p>}
    {equipment.error&&<p className="notice notice-warning" role="alert">No se pudieron actualizar los equipos. <button className="button" onClick={equipment.refresh}>Reintentar equipos</button></p>}
    <section className="fleet-kpis" aria-label="Resumen global de hoy">{[
      ['Equipos con actividad hoy', overview?.devices_today], ['Pulsaciones registradas', overview?.keys_today], ['Clics registrados', overview?.mouse_clicks_today], ['Movimientos registrados', overview?.mouse_moves_today],
    ].map(([label, value]) => <article className="fleet-kpi" key={String(label)}><div className="kpi-heading">{label}</div><strong className="kpi-value">{typeof value === 'number' ? value.toLocaleString() : '—'}</strong><p>Resumen global del día · según datos recibidos</p></article>)}</section>
    <section className="panel"><div className="panel-header"><div><p className="eyebrow">Explorar por equipo</p><h2>Actividad registrada</h2></div>{deviceId && <Link className="text-link" to={`/devices/${encodeURIComponent(deviceId)}?tab=resources&date=${date}`}>Abrir equipo <Icon name="arrow" /></Link>}</div><div className="fleet-toolbar"><label className="metric-selector">Equipo<select value={deviceId} onChange={e => setDeviceId(e.target.value)} disabled={!devices.length}>{!devices.length && <option value="">Sin equipos disponibles</option>}{devices.map(d => <option key={d.device_id} value={d.device_id}>{d.nickname || d.hostname}</option>)}</select></label><label className="metric-selector">Día UTC<input type="date" value={date} max={today()} onChange={e => { if (e.target.value) setDate(e.target.value); }} /></label></div><p className="panel-note">Detalle horario en UTC. Las barras muestran minutos registrados; ausencia de actividad no confirma que el recolector esté disponible.</p>
      {hourlyError&&<p className="notice notice-error" role="alert">{hourlyError} <button className="button" onClick={hourlyState.refresh}>Reintentar actividad horaria</button></p>}
      {!deviceId ? <div className="empty-state">Selecciona un equipo para consultar su actividad.</div> : hourly === null ? <div className="empty-state" role="status">Cargando actividad…</div> : hourly.length === 0 ? <div className="empty-state"><h3>Sin muestras para este día</h3><p>Selecciona otra fecha o consulta los resúmenes diarios.</p></div> : <div className="metric-charts">{[['Actividad', 'active_seconds', 'var(--palette-508967)'], ['Inactividad', 'idle_seconds', 'var(--palette-b19558)']].map(([title, key, color]) => <div key={key}><h3>{title} · minutos por hora</h3><SimpleBarChart data={hourly.map(row => ({ ...row, [key]: Math.round(row[key as 'active_seconds' | 'idle_seconds'] / 60 * 10) / 10 }))} dataKey={key} label={`${title} en minutos`} color={color} /></div>)}</div>}
    </section>
    {deviceId && <DailyMetricsPanel key={deviceId} deviceId={deviceId} endDate={date} />}
    {apiClient.getRole()==='admin'&&<MaintenancePanel />}
  </AppShell>;
}
