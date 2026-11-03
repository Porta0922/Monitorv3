import { useEffect, useState } from 'react';
import { apiClient } from '../../api/client';
import type { StoragePage } from '../../api/client';
export function StoragePanel() {
  const [mode,setMode]=useState('hypertables');
  const [offset,setOffset]=useState(0);
  const [revision,setRevision]=useState(0);
  const [data,setData]=useState<StoragePage | null>(null);
  const [error,setError]=useState('');
  useEffect(()=>{const abort=new AbortController();apiClient.getStorage(mode,offset,abort.signal).then(d=>{if(!abort.signal.aborted)setData(d);}).catch(()=>{if(!abort.signal.aborted)setError('No se pudo consultar el almacenamiento.');});return ()=>abort.abort();},[mode,offset,revision]);
  const reset=()=>{setData(null);setError('');};
  return <section className="rounded-xl border p-4 text-sm"><h3>Almacenamiento</h3>
    <div className="flex flex-wrap gap-3"><label>Vista<select value={mode} onChange={e=>{reset();setMode(e.target.value);setOffset(0);}}><option value="hypertables">Históricos: total por hypertable</option><option value="relations">Tablas y relaciones físicas</option></select></label><button className="button" onClick={()=>{reset();setRevision(v=>v+1);}}>Actualizar tamaños</button></div>
    <p>{mode==='hypertables' ? 'Totales calculados por Timescale incluyendo chunks e índices. No sumes estos totales a la vista de relaciones físicas.' : 'Relaciones físicas en orden alfabético; las filas son estimaciones de PostgreSQL. La lista puede cambiar entre páginas.'}</p>
    {error ? <p className="notice notice-error" role="alert">{error}</p> : !data ? <p>Cargando tamaños…</p> : <>
      <p>Consulta: {new Date(data.observed_at).toLocaleString()}</p><div className="overflow-x-auto"><table className="w-full text-left"><thead><tr><th>Relación</th><th>Total</th><th>Índices</th><th>Filas estimadas</th></tr></thead><tbody>{data.items.map(r=><tr key={`${r.schema_name}.${r.relation_name}`}><td>{r.schema_name}.{r.relation_name}</td><td>{(r.total_bytes/1048576).toFixed(1)} MiB</td><td>{(r.index_bytes/1048576).toFixed(1)} MiB</td><td>{r.estimated_rows ?? 'Sin estimación'}</td></tr>)}</tbody></table></div>
      {!data.items.length && <p>No hay relaciones en esta página.</p>}
      <div className="flex items-center gap-3"><button className="button" disabled={!offset} onClick={()=>{reset();setOffset(v=>Math.max(0,v-50));}}>Anterior</button><span>Página {offset/50+1}</span><button className="button" disabled={!data.has_more} onClick={()=>{reset();setOffset(v=>v+50);}}>Siguiente</button></div>
    </>}
  </section>;
}
