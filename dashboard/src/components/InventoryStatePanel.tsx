import { useEffect, useState } from 'react';
import { apiClient } from '../api/client';
import type { InventoryStatePage } from '../types';
const sourceName: Record<string,string> = { 'linux-dpkg': 'Paquetes Debian/Ubuntu', 'windows-machine-64': 'Windows · registro 64 bits', 'windows-machine-32': 'Windows · registro 32 bits' };
const actionName = { added: 'Alta', updated: 'Cambio', removed: 'Baja' };
export function InventoryStatePanel({ deviceId }: { deviceId: string }) {
  const [mode,setMode]=useState<'current'|'changes'>('current');
  const [search,setSearch]=useState(''); const [query,setQuery]=useState('');
  const [offset,setOffset]=useState(0); const [revision,setRevision]=useState(0);
  const [data,setData]=useState<InventoryStatePage|null>(null); const [error,setError]=useState(false); const [loading,setLoading]=useState(true);
  const beginLoad=()=>{setLoading(true);setError(false);setData(null);};
  useEffect(()=>{
    const controller=new AbortController();
    apiClient.getInventoryState(deviceId,mode,query,offset,controller.signal).then(next=>{if(!controller.signal.aborted)setData(next);})
      .catch(()=>{if(!controller.signal.aborted)setError(true);})
      .finally(()=>{if(!controller.signal.aborted)setLoading(false);});
    return ()=>controller.abort();
  },[deviceId,mode,query,offset,revision]);
  return <section className="panel">
    <div className="panel-header"><h2>Software por fuente</h2><button className="button" disabled={loading} onClick={()=>{beginLoad();setRevision(v=>v+1);}}>Actualizar</button></div>
    <div className="p-4 space-y-3">
      <p className="muted">Último estado confirmado de las fuentes listadas. No incluye aplicaciones portables, Snap/Flatpak ni instalaciones por usuario fuera del registro del sistema.</p>
      <div className="flex flex-wrap gap-3"><label>Vista <select value={mode} onChange={e=>{beginLoad();setMode(e.target.value as 'current'|'changes');setOffset(0);}}><option value="current">Estado actual</option><option value="changes">Cambios registrados</option></select></label>
        <form onSubmit={e=>{e.preventDefault();beginLoad();setQuery(search.trim());setOffset(0);setRevision(v=>v+1);}} className="flex gap-2"><label><span className="sr-only">Buscar aplicación</span><input maxLength={256} value={search} onChange={e=>setSearch(e.target.value)} placeholder="Buscar aplicación" /></label><button className="button" type="submit">Buscar</button></form></div>
      {loading && <p role="status">Consultando inventario…</p>}
      {error && <p role="alert" className="notice notice-warning">No se pudo consultar el inventario. Reintenta con Actualizar.</p>}
      {data && <>
        {!data.sources.length ? <p className="notice">Aún no hay escaneos del nuevo contrato. Las detecciones heredadas no prueban qué está instalado actualmente.</p> : <ul>{data.sources.map(source=><li key={source.scope}><strong>{sourceName[source.scope]??source.scope}</strong>: {source.last_error ? `escaneo incompleto: ${source.last_error}` : source.stale ? 'sin escaneo completo reciente' : 'último escaneo completo'} · {source.last_complete_at ? new Date(source.last_complete_at).toLocaleString() : 'sin estado confirmado'} · {source.item_count??'—'} entradas</li>)}</ul>}
        {mode==='changes' && <p className="muted">Las altas iniciales forman la línea base. Las bajas solo provienen de un escaneo completo; el historial depende de la retención configurada.</p>}
        <div className="table-scroll"><table className="w-full text-left"><thead><tr><th>Aplicación</th><th>Versión</th><th>Fuente</th><th>{mode==='changes'?'Cambio':'Último cambio'}</th></tr></thead><tbody>
          {data.items.map(row=><tr key={`${row.scope}:${row.source_key}:${row.id??''}`}><td>{row.app_name??row.after_value?.app_name??row.before_value?.app_name}</td><td>{mode==='changes' ? `${row.before_value?.version??'—'} → ${row.after_value?.version??'—'}` : row.version??'Sin informar'}</td><td>{sourceName[row.scope]??row.scope}</td><td>{row.action ? `${actionName[row.action]} · ` : ''}{row.timestamp||row.changed_at ? new Date(row.timestamp??row.changed_at??'').toLocaleString() : '—'}</td></tr>)}
          {!data.items.length && <tr><td colSpan={4}>No hay entradas para esta consulta. Revisa la cobertura y fecha del escaneo.</td></tr>}
        </tbody></table></div>
        <div className="table-footer"><span>{data.total} registros · búsqueda y paginación en servidor</span><div><button className="button" disabled={offset===0||loading} onClick={()=>{beginLoad();setOffset(v=>Math.max(0,v-50));}}>Anterior</button><span>Página {offset/50+1}</span><button className="button" disabled={offset+data.items.length>=data.total||loading} onClick={()=>{beginLoad();setOffset(v=>v+50);}}>Siguiente</button></div></div>
      </>}
    </div>
  </section>;
}
