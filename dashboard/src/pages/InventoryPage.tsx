import { useEffect, useState } from 'react';
import { AppShell } from '../components/AppShell';
import { InventoryStatePanel } from '../components/InventoryStatePanel';
import { apiClient } from '../api/client';
import type { Device } from '../types';
export function InventoryPage() {
  const [devices,setDevices]=useState<Device[]>([]); const [selected,setSelected]=useState('');
  const [loading,setLoading]=useState(true); const [error,setError]=useState(false); const [revision,setRevision]=useState(0);
  useEffect(()=>{let alive=true;apiClient.getDevices().then(items=>{if(alive){setDevices(items);setSelected(previous=>items.some(d=>d.device_id===previous)?previous:items[0]?.device_id??'');}}).catch(()=>{if(alive)setError(true);}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[revision]);
  return <AppShell currentPage="inventory" title="Inventario de software" subtitle="Estado confirmado por fuente y cambios registrados por equipo.">
    {error ? <p role="alert" className="notice notice-warning">No se pudieron cargar los equipos. <button className="button" onClick={()=>{setLoading(true);setError(false);setRevision(v=>v+1);}}>Reintentar</button></p> : loading ? <p role="status">Consultando equipos…</p> : !devices.length ? <p>No hay equipos registrados.</p> : <label>Equipo <select value={selected} onChange={e=>setSelected(e.target.value)}>{devices.map(d=><option key={d.device_id} value={d.device_id}>{d.nickname||d.hostname||d.device_id}</option>)}</select></label>}
    {!error && selected && <InventoryStatePanel key={selected} deviceId={selected} />}
  </AppShell>;
}
