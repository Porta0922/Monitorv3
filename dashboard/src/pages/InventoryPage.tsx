import {AppShell} from '../components/AppShell';
import {InventoryStatePanel} from '../components/InventoryStatePanel';
import {apiClient} from '../api/client';
import {useRemoteSection} from '../hooks/useRemoteSection';
import {useQueryState} from '../hooks/useQueryState';
const loadDevices=()=>apiClient.getDevices();
export function InventoryPage() {
  const {data:devices,error,loading,refresh}=useRemoteSection(loadDevices);
  const [requested,setSelected]=useQueryState('device','',['software-page']);
  const selected=requested||devices?.[0]?.device_id||'';
  const exists=devices?.some(d=>d.device_id===selected);
  return <AppShell currentPage="inventory" title="Inventario de software" subtitle="Estado confirmado por fuente y cambios registrados por equipo.">
    {error&&<p role="alert" className="notice notice-warning">No se pudieron actualizar los equipos. <button className="button" onClick={refresh}>Reintentar</button></p>}
    {loading&&!devices?<p role="status">Consultando equipos…</p>:devices?.length?<label>Equipo <select value={selected} onChange={e=>setSelected(e.target.value)}>{!exists&&<option value={selected}>Equipo no disponible</option>}{devices.map(d=><option key={d.device_id} value={d.device_id}>{d.nickname||d.hostname||d.device_id}</option>)}</select></label>:!error&&<p>No hay equipos registrados.</p>}
    {exists&&<InventoryStatePanel key={selected} deviceId={selected}/>}
  </AppShell>;
}
