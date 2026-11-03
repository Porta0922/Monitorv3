import {useRemoteSection} from '../../hooks/useRemoteSection';
import {useCallback,useState} from 'react';
import {apiClient} from '../../api/client';
import type {SessionActivityPage} from '../../api/client';
export function SessionActivityPanel({deviceId,compact=false,initialDay}:{deviceId:string;compact?:boolean;initialDay?:string}) {
  const [day,setDay]=useState(()=>initialDay??new Date().toISOString().slice(0,10));
  const [offset,setOffset]=useState(0);
  const [exportError,setExportError]=useState(false);const [exporting,setExporting]=useState(false);
  const {data:value,error,loading,refresh}=useRemoteSection<SessionActivityPage>(useCallback(()=>apiClient.getSessionActivity(deviceId,day,offset),[deviceId,day,offset]));
  const duration=(seconds:number|null)=>seconds==null?'Sin muestras disponibles':`${(seconds/3600).toLocaleString('es',{maximumFractionDigits:2})} h`;
  async function exportDay(){setExporting(true);setExportError(false);try{const blob=await apiClient.exportSessionActivity(deviceId,day);const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download=`monitor-sesiones-${day}.csv`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}catch{setExportError(true);}finally{setExporting(false);}}
  const count=(n:number|null|undefined)=>n==null?'Sin datos':n.toLocaleString('es');
  return <section className="panel p-5" aria-label="Actividad por sesión"><div className="panel-header"><h2>Actividad de personas y sesiones</h2><button className="button" disabled={loading} onClick={refresh}>Actualizar sesiones</button></div>
    {!compact && <label>Día UTC <input type="date" required value={day} onChange={e=>{if(e.target.value){setDay(e.target.value);setOffset(0);}}}/></label>}
    <details><summary>Cómo interpretar los contadores</summary><p className="muted">Datos de enrolamientos con credenciales individuales. UID/SID identifica al principal; sesión identifica una ejecución del agente. Los tiempos son por persona y no representan horas de encendido del equipo. Los contadores corresponden a muestras con captura disponible. Los totales del día incluyen todas las páginas; no se suman con los contadores heredados.</p></details>
    {apiClient.getRole()!=='viewer'&&<button className="button" disabled={exporting||!value} onClick={()=>void exportDay()}>{exporting?'Exportando…':'Exportar contadores del día'}</button>}
    {exportError&&<p role="alert">No se pudo exportar el día. No se descargaron datos parciales.</p>}
    {loading&&<p role="status">Consultando sesiones…</p>}
    {error&&<p role="alert" className="notice notice-warning">No se pudo actualizar sesiones. Los datos anteriores, si existen, pueden estar desactualizados. <button className="button" onClick={refresh} disabled={loading}>Reintentar sesiones</button></p>}
    {value&&<div className="notice"><strong>Totales del día UTC</strong><p>Actividad: {duration(value.totals.active_person_seconds)} · Inactividad: {duration(value.totals.idle_person_seconds)}</p><p>Teclas: {count(value.totals.keys_count)} · Movimientos: {count(value.totals.mouse_moves_count)} · Clics: {count(value.totals.clicks_count)}</p><p>Muestras: {value.totals.available_samples} disponibles · {value.totals.unavailable_samples} sin captura · {value.totals.unknown_samples} sin estado informado.</p></div>}
    <details open={compact?undefined:true}><summary>Desglose por persona y sesión</summary>
    {!value?<p role="status">Consultando…</p>:!value.items.length?<p>No hay actividad de sesiones para esta página y día.</p>:<div className="table-scroll" tabIndex={0} role="region" aria-label="Contadores por sesión"><table><thead><tr><th scope="col">Principal</th><th scope="col">Sesión</th><th scope="col">Actividad</th><th scope="col">Inactividad</th><th scope="col">Teclas / movimientos / clics</th><th scope="col">Muestras disponibles / no disponibles / desconocidas</th><th scope="col">Eventos</th></tr></thead><tbody>{value.items.map(s=><tr key={`${s.principal}-${s.session_id}`}><td>{s.principal}</td><td title={s.session_id}>{s.session_id.slice(0,8)}</td><td>{duration(s.active_person_seconds)}</td><td>{duration(s.idle_person_seconds)}</td><td>{count(s.keys_count)} / {count(s.mouse_moves_count)} / {count(s.clicks_count)}</td><td>{s.available_samples} / {s.unavailable_samples} / {s.unknown_samples}</td><td>{s.events}</td></tr>)}</tbody></table></div>}
    <div className="table-footer"><button className="button" disabled={loading||offset===0} onClick={()=>setOffset(v=>Math.max(0,v-50))}>Anterior</button><span>{value?`${value.total} sesiones en el día UTC`:'Sin recuento'}</span><button className="button" disabled={loading||!value||offset+50>=value.total} onClick={()=>setOffset(v=>v+50)}>Siguiente</button></div>
    </details>
  </section>;
}
