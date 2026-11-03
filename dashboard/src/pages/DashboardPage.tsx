import {OverviewCard} from '../components/OverviewCard';
import { useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { Icon } from '../components/Icon';
import { DeviceActionsDialog } from '../components/DeviceActionsDialog';
import { useFleet } from '../hooks/useFleet';
import { apiClient } from '../api/client';
import type { Device } from '../types';

const healthOf = (d: Device) => d.agent_health_error ? 'Error al consultar' : !d.agent_health ? 'Sin diagnóstico' : d.agent_health.stale ? 'Diagnóstico atrasado' : d.agent_health.monitoring_paused ? 'Pausado' : d.agent_health.status === 'degraded' ? 'Degradado' : d.agent_health.status === 'healthy' ? 'Operativo' : 'Desconocido';
const nameOf = (d: Device) => d.nickname || d.hostname || d.device_id;
const stateOf = (d: Device) => d.stale ? 'stale' : d.online ? 'online' : 'offline';
const stateNames: Record<string, string> = { online: 'Conectado', offline: 'Desconectado', stale: 'Señal atrasada' };
const dateOf = (value: string) => Number.isFinite(Date.parse(value)) ? new Date(value).toLocaleString('es', { dateStyle: 'short', timeStyle: 'short' }) : 'Sin registro';
const percent = (value: number) => Number.isFinite(value) ? `${value.toFixed(1)}%` : 'Sin dato';
const severityOrder: Record<string, number> = { CRITICAL: 0, HIGH: 1, MEDIUM: 2, LOW: 3 };
const severityNames: Record<string, string> = { CRITICAL: 'Crítica', HIGH: 'Alta', MEDIUM: 'Media', LOW: 'Baja' };

export function DashboardPage() {
  const [params, setParams] = useSearchParams();
  const [managed, setManaged] = useState<Device | null>(null);
  const query = params.get('q') ?? '';
  const requestedStatus = params.get('status') ?? 'all';
  const status = ['all', 'online', 'offline', 'stale'].includes(requestedStatus) ? requestedStatus : 'all';
  const sort = params.get('sort') === 'recent' ? 'recent' : 'name';
  const pageNumber = Number(params.get('page') ?? 1);
  const page = Number.isSafeInteger(pageNumber) && pageNumber > 0 ? Math.min(pageNumber,50001) : 1;
  const change = (key: string, value: string) => { const next = new URLSearchParams(params); if (value) next.set(key, value); else next.delete(key); if (key !== 'page') next.delete('page'); setParams(next, { replace: true }); };
  const group=params.get('group')??'';
  const deviceLink = (id: string) => `/devices/${encodeURIComponent(id)}?${new URLSearchParams({equipmentQuery:params.toString()})}`;
  const requestQuery=new URLSearchParams({q:query,status,sort,group,page:String(Math.min(page,50001))}).toString();
  const {devices,total,counts,groups,alerts,peaks,errors,updatedAt,refreshing,refresh}=useFleet(requestQuery);
  const [columns,setColumns]=useState<string[]>(()=>{try{const value=JSON.parse(localStorage.getItem('monitor-equipment-columns')??'null');return Array.isArray(value)&&value.every(v=>typeof v==='string')?value:['health','last_seen','identity','group','version'];}catch{return ['health','last_seen','identity','group','version'];}});
  const toggleColumn=(column:string)=>{const next=columns.includes(column)?columns.filter(c=>c!==column):[...columns,column];setColumns(next);try{localStorage.setItem('monitor-equipment-columns',JSON.stringify(next));}catch{/* Preferences remain available in memory. */}};
  const pages=Math.max(1,Math.ceil(total/20)); const currentPage=page; const shown=devices??[];
  const fleetKnown=counts!==null&&!errors.includes('equipos');
  const alertsKnown=alerts!==null&&!errors.includes('alertas');
  const peaksKnown=peaks!==null&&!errors.includes('recursos');
  const online=counts?.online??0,stale=counts?.stale??0,offline=counts?.offline??0;
  const sortedAlerts = [...(alerts ?? [])].sort((a, b) => (severityOrder[a.severity] ?? 9) - (severityOrder[b.severity] ?? 9));
  const deviceNames = new Map((devices ?? []).map(d => [d.device_id, nameOf(d)]));
  return <AppShell currentPage="dashboard" title="Equipos de oficina" subtitle="Conectividad, alertas y recursos para decidir dónde intervenir." actions={<button className="button" disabled={refreshing} onClick={refresh}><Icon name="refresh" />{refreshing ? 'Actualizando…' : 'Actualizar'}</button>}>
    <div className="freshness"><span className={`status-dot ${errors.length ? 'warning' : updatedAt ? 'success' : ''}`} /><span>{updatedAt ? `Equipos consultados a las ${updatedAt.toLocaleTimeString('es', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}` : 'Esperando primera consulta'}</span><span className="freshness-divider">·</span><span>Actualización cada 30 s mientras esta pestaña está visible</span></div>
    {errors.length > 0 && <div role="alert" className="notice notice-warning"><strong>No se pudo actualizar: {errors.join(', ')}.</strong> Los datos anteriores, si existen, pueden estar desactualizados. <button className="text-button" onClick={refresh} disabled={refreshing}>Reintentar</button></div>}
    <section className="fleet-kpis" aria-label="Resumen de los equipos">
      {[
        { title: 'Equipos registrados', value: fleetKnown ? counts!.total : '—', note: 'Todos los equipos de la organización', icon: 'monitor' as const, filter: 'all' },
        { title: 'Conectados', value: fleetKnown ? online : '—', note: 'Sin señal marcada como atrasada', icon: 'activity' as const, filter: 'online' },
        { title: 'Sin señal reciente', value: fleetKnown ? offline + stale : '—', note: fleetKnown ? `${offline} desconectados · ${stale} con señal atrasada` : 'Estado no disponible', icon: 'wifi' as const, filter: 'offline' },
        { title: 'Alertas abiertas', value: alertsKnown ? alerts.length : '—', note: 'Pendientes de revisión o resolución', icon: 'shield' as const, filter: 'alerts' },
      ].map((item, index) => <article className={`fleet-kpi kpi-${index}`} key={item.title}><div className="kpi-heading"><span>{item.title}</span><Icon name={item.icon} /></div><strong className="kpi-value">{item.value}</strong><p>{item.note}</p>{item.filter === 'alerts' ? <Link className="kpi-link" to="/alerts">Revisar alertas <Icon name="arrow" /></Link> : <button className="kpi-link" onClick={() => change('status', item.filter)}>{index === 2 ? 'Ver desconectados' : 'Ver equipos'}<Icon name="arrow" /></button>}</article>)}
    </section>
    <section className="panel fleet-panel" aria-labelledby="fleet-title">
      <div className="panel-header"><div><p className="eyebrow">Inventario operativo</p><h2 id="fleet-title">Equipos</h2></div><span className="muted">{devices === null ? 'Sin datos todavía' : `${total} encontrados`}</span></div>
      <div className="fleet-toolbar"><label className="search-field"><Icon name="search" /><span className="sr-only">Buscar equipos</span><input type="search" placeholder="Buscar equipo, nombre, MAC o identificador…" value={query} onChange={event => change('q', event.target.value)} /></label><label className="sort-field">Ordenar<select value={sort} onChange={event => change('sort', event.target.value)}><option value="name">Nombre del equipo</option><option value="recent">Última conexión</option></select></label></div>
      <div className="fleet-toolbar"><label>Grupo<select value={group} onChange={e=>change('group',e.target.value)}><option value="">Todos los grupos</option>{groups.map(g=><option key={g} value={g}>{g}</option>)}</select></label><details><summary>Columnas visibles</summary><div className="flex flex-wrap gap-3">{[['health','Salud'],['last_seen','Última señal'],['identity','Identificación'],['group','Grupo'],['version','Versión']].map(([key,label])=><label key={key}><input type="checkbox" checked={columns.includes(key)} onChange={()=>toggleColumn(key)}/>{label}</label>)}</div></details><Link className="text-link" to="/administration">Incorporación y políticas</Link></div>
      <OverviewCard/>
      <div className="filter-tabs" aria-label="Filtrar por conexión">{[['all', 'Todos'], ['online', 'Conectados'], ['offline', 'Desconectados'], ['stale', 'Señal atrasada']].map(([key, label]) => <button key={key} aria-pressed={status === key} className={status === key ? 'active' : ''} onClick={() => change('status', key)}>{label}</button>)}</div>
      {devices === null ? <div className="empty-state" role="status"><Icon name="monitor" /><h3>{refreshing ? 'Cargando equipos…' : 'No se pudieron consultar los equipos'}</h3><p>{refreshing ? 'Estamos consultando los equipos registrados.' : 'Reintenta la consulta para recuperar el estado de los equipos.'}</p></div>
        : (counts?.total??0) === 0 ? <div className="empty-state"><Icon name="monitor" /><h3>Tu primer equipo empieza aquí</h3><p>Instala y configura un agente para que el equipo aparezca en esta vista al enviar su primera señal.</p><button className="button" onClick={refresh} disabled={refreshing}>Volver a consultar</button></div>
        : shown.length === 0 ? <div className="empty-state"><Icon name="search" /><h3>No hay equipos con estos filtros</h3><p>Prueba otro nombre o amplía el estado de conexión.</p><button className="button" onClick={() => setParams({})}>Limpiar filtros y volver al inicio</button></div>
        : <div className="table-scroll" tabIndex={0} role="region" aria-label="Listado de equipos"><table className="fleet-table"><thead><tr><th scope="col">Equipo</th><th scope="col">Conexión</th>{columns.includes('health')&&<th scope="col">Salud del agente</th>}{columns.includes('last_seen')&&<th scope="col">Última señal</th>}{columns.includes('identity')&&<th scope="col">Identificación</th>}{columns.includes('group')&&<th scope="col">Grupo</th>}{columns.includes('version')&&<th scope="col">Versión / objetivo</th>}<th scope="col"><span className="sr-only">Acciones</span></th></tr></thead><tbody>{shown.map(device => <tr key={device.device_id}>
          <td><div className="device-cell"><span className="device-symbol"><Icon name="monitor" /></span><div><Link className="device-name" to={deviceLink(device.device_id)}>{nameOf(device)}</Link><span className="cell-secondary">{device.nickname ? device.hostname : 'Equipo administrado'}</span></div></div></td>
          <td><span className={`status-badge ${stateOf(device)}`}><span className="status-dot" />{stateNames[stateOf(device)]}</span></td>
          {columns.includes('health')&&<td><span className={device.agent_health?.status === 'degraded' && !device.agent_health.stale ? 'notice-warning' : 'muted'}>{healthOf(device)}</span></td>}
          {columns.includes('last_seen')&&<td><time dateTime={device.last_seen}>{dateOf(device.last_seen)}</time><span className="cell-secondary">Hora local del navegador</span></td>}
          {columns.includes('identity')&&<td><span className="identifier">{device.mac_address || 'MAC no disponible'}</span><span className="cell-secondary identifier" title={device.device_id}>{device.device_id.slice(0, 8)}…</span></td>}
          {columns.includes('group')&&<td>{device.group_name||'Sin grupo'}</td>}{columns.includes('version')&&<td>{device.version||'Sin informar'}<span className="cell-secondary">{device.desired_version?`Objetivo: ${device.desired_version}${device.version===device.desired_version?' · coincide':' · pendiente'}`:'Sin objetivo'}</span></td>}
          <td><div className="row-actions"><Link className="button button-small" to={deviceLink(device.device_id)}>Ver detalle<Icon name="arrow" /></Link>{apiClient.getRole()==='admin'&&<button className="icon-button" aria-label={`Administrar ${nameOf(device)}`} title="Administrar equipo" onClick={() => setManaged(device)}>•••</button>}</div></td>
        </tr>)}</tbody></table></div>}
      {devices !== null && total > 0 && <div className="table-footer"><span>{shown.length?(currentPage - 1) * 20 + 1:0}–{shown.length?Math.min(currentPage * 20, total):0} de {total} equipos</span><div><button className="button button-small" disabled={currentPage === 1} onClick={() => change('page', String(currentPage - 1))}>Anterior</button><span aria-live="polite">Página {currentPage} de {pages}</span><button className="button button-small" disabled={currentPage >= pages} onClick={() => change('page', String(currentPage + 1))}>Siguiente</button></div></div>}
    </section>
    <div className="insights-grid">
      <section className="panel" aria-labelledby="attention-title"><div className="panel-header"><div><p className="eyebrow">Prioridad operativa</p><h2 id="attention-title">Requiere atención</h2></div><Link className="text-link" to="/alerts">Ver alertas <Icon name="arrow" /></Link></div>
        {!alertsKnown ? <div className="empty-state compact"><p>{refreshing ? 'Consultando alertas…' : 'Alertas no disponibles. No se puede confirmar su estado.'}</p></div> : sortedAlerts.length === 0 ? <div className="empty-state compact"><span className="empty-icon-success"><Icon name="shield" /></span><h3>Sin alertas abiertas</h3><p>No hay alertas pendientes en la última consulta.</p></div> : <div className="attention-list">{sortedAlerts.slice(0, 4).map(alert => <Link to={deviceLink(alert.device_id)} className="attention-item" key={alert.id}><div><span className={`severity severity-${alert.severity.toLowerCase()}`}>{severityNames[alert.severity] ?? alert.severity}</span><span className="muted">{deviceNames.get(alert.device_id) ?? alert.device_id.slice(0, 8)}</span></div><strong>{alert.alert_type.replaceAll('_', ' ')}</strong><p>{alert.description}</p></Link>)}</div>}
      </section>
      <section className="panel" aria-labelledby="resource-title"><div className="panel-header"><div><p className="eyebrow">Recursos de hoy</p><h2 id="resource-title">Picos de utilización</h2></div><Link className="text-link" to="/metrics">Métricas <Icon name="arrow" /></Link></div><p className="panel-note">Máximos diarios de CPU y memoria. No representan el consumo actual. Hasta 20 equipos reportados por la API.</p>
        {!peaksKnown ? <div className="empty-state compact"><p>{refreshing ? 'Consultando recursos…' : 'No se pudieron consultar los recursos.'}</p></div> : peaks.length === 0 ? <div className="empty-state compact"><Icon name="chart" /><h3>Todavía no hay muestras</h3><p>Los recursos aparecerán cuando los agentes envíen métricas.</p></div> : <div className="resource-list">{peaks.slice(0, 4).map(peak => <Link className="resource-item" to={deviceLink(peak.device_id)} key={peak.device_id}><div className="resource-name"><strong>{deviceNames.get(peak.device_id) ?? peak.device_id.slice(0, 8)}</strong><Icon name="arrow" /></div><div className="resource-measures">{[['CPU', peak.peak_cpu_percent], ['Memoria', peak.peak_memory_percent]].map(([label, value]) => <div key={label}><span>{label}<b>{percent(Number(value))}</b></span><div className="meter" aria-hidden="true"><i style={{ width: `${Math.min(100, Math.max(0, Number(value) || 0))}%` }} /></div></div>)}</div></Link>)}</div>}
      </section>
    </div>
    {managed && <DeviceActionsDialog key={managed.device_id} device={managed} onClose={() => setManaged(null)} onChanged={refresh} />}
  </AppShell>;
}
