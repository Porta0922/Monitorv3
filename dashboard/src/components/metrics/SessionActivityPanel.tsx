import {useEffect,useState} from 'react';
import {apiClient} from '../../api/client';
import type {SessionActivityPage} from '../../api/client';
export function SessionActivityPanel({deviceId,compact=false,initialDay}:{deviceId:string;compact?:boolean;initialDay?:string}) {
  const [day,setDay]=useState(()=>initialDay??new Date().toISOString().slice(0,10));
  const [offset,setOffset]=useState(0);const [value,setValue]=useState<SessionActivityPage|null>(null);
  const [exportError,setExportError]=useState(false);const [exporting,setExporting]=useState(false);
  const [error,setError]=useState(false);const [revision,setRevision]=useState(0);
  useEffect(()=>{const abort=new AbortController();setValue(null);setError(false);
    apiClient.getSessionActivity(deviceId,day,offset,abort.signal).then(v=>{if(!abort.signal.aborted)setValue(v);}).catch(()=>{if(!abort.signal.aborted)setError(true);});
    return()=>abort.abort();},[deviceId,day,offset,revision]);
  const duration=(seconds:number|null)=>seconds==null?'Sin muestras disponibles':`${(seconds/3600).toLocaleString('es',{maximumFractionDigits:2})} h`;
  async function exportDay(){setExporting(true);setExportError(false);try{const blob=await apiClient.exportSessionActivity(deviceId,day);const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download=`monitor-sesiones-${day}.csv`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}catch{setExportError(true);}finally{setExporting(false);}}
  const count=(n:number|null|undefined)=>n==null?'Sin datos':n.toLocaleString('es');
  return <section className="panel p-5" aria-label="Actividad por sesión"><div className="panel-header"><h2>Actividad de personas y sesiones</h2><button className="button" onClick={()=>setRevision(v=>v+1)}>Actualizar sesiones</button></div>
    <label>Día UTC <input type="date" required value={day} onChange={e=>{if(e.target.value){setDay(e.target.value);setOffset(0);}}}/></label>
    <p className="muted">Datos de enrolamientos con credenciales individuales. UID/SID identifica al principal; sesión identifica una ejecución del agente. Los tiempos son por persona y no representan horas de encendido del equipo. Los contadores corresponden a muestras con captura disponible. Los totales del día incluyen todas las páginas; no se suman con los contadores heredados.</p>
    {apiClient.getRole()!=='viewer'&&<button className="button" disabled={exporting||!value} onClick={()=>void exportDay()}>{exporting?'Exportando…':'Exportar contadores del día'}</button>}
    {exportError&&<p role="alert">No se pudo exportar el día. No se descargaron datos parciales.</p>}
    {value&&<div className="notice"><strong>Totales del día UTC</strong><p>Actividad: {duration(value.totals.active_person_seconds)} · Inactividad: {duration(value.totals.idle_person_seconds)}</p><p>Teclas: {count(value.totals.keys_count)} · Movimientos: {count(value.totals.mouse_moves_count)} · Clics: {count(value.totals.clicks_count)}</p><p>Muestras: {value.totals.available_samples} disponibles · {value.totals.unavailable_samples} sin captura · {value.totals.unknown_samples} sin estado informado.</p></div>}
    <details open={compact?undefined:true}><summary>Desglose por persona y sesión</summary>
    {error?<p role="alert">No se pudo consultar la actividad de sesiones.</p>:!value?<p role="status">Consultando…</p>:!value.items.length?<p>No hay actividad de sesiones para esta página y día.</p>:<div className="table-scroll"><table><thead><tr><th>Principal</th><th>Sesión</th><th>Actividad</th><th>Inactividad</th><th>Teclas / movimientos / clics</th><th>Muestras disponibles / no disponibles / desconocidas</th><th>Eventos</th></tr></thead><tbody>{value.items.map(s=><tr key={`${s.principal}-${s.session_id}`}><td>{s.principal}</td><td title={s.session_id}>{s.session_id.slice(0,8)}</td><td>{duration(s.active_person_seconds)}</td><td>{duration(s.idle_person_seconds)}</td><td>{count(s.keys_count)} / {count(s.mouse_moves_count)} / {count(s.clicks_count)}</td><td>{s.available_samples} / {s.unavailable_samples} / {s.unknown_samples}</td><td>{s.events}</td></tr>)}</tbody></table></div>}
    <div className="table-footer"><button className="button" disabled={offset===0} onClick={()=>setOffset(v=>Math.max(0,v-50))}>Anterior</button><span>{value?`${value.total} sesiones en el día UTC`:'Sin recuento'}</span><button className="button" disabled={!value||offset+50>=value.total} onClick={()=>setOffset(v=>v+50)}>Siguiente</button></div>
    </details>
  </section>;
}
