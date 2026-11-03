import {useCallback,useEffect} from 'react';
import {Link} from 'react-router-dom';
import {apiClient} from '../api/client';
import {useRemoteSection} from '../hooks/useRemoteSection';
import {CollectionTable} from '../components/CollectionTable';
import {AppShell} from '../components/AppShell';
interface Recent {device_id:string;app:string;title:string;last_seen:string;is_stale:boolean}
const loadDevices=()=>apiClient.getDevices();
export function ActivityPage() {
  const devices=useRemoteSection(loadDevices);
  const activity=useRemoteSection(useCallback(async()=>await apiClient.getLiveDevices({limit:100}) as Recent[],[]));
  const {loading,refresh}=activity;
  useEffect(()=>{if(loading)return;const timer=setInterval(()=>{if(!document.hidden)refresh();},10000);return()=>clearInterval(timer);},[loading,refresh]);
  const names=new Map(devices.data?.map(d=>[d.device_id,d.nickname||d.hostname]));
  return <AppShell currentPage="activity" title="Actividad reciente" subtitle="Última aplicación reportada por equipo; no es una vista en vivo del escritorio." actions={<button className="button" onClick={activity.refresh} disabled={activity.loading}>Actualizar actividad</button>}>
    {devices.error&&<p className="notice notice-warning" role="alert">No se pudieron actualizar los nombres; se muestran identificadores cuando faltan. <button className="button" onClick={devices.refresh}>Reintentar nombres</button></p>}
    <CollectionTable title="Actividad reciente por equipo" rows={activity.data} loading={activity.loading} error={activity.error} refresh={activity.refresh}
      note={`Hasta 100 registros por consulta, actualización cada 10 s mientras la pestaña está visible. ${activity.updatedAt?`Consulta: ${activity.updatedAt.toLocaleString()}.`:''} Horarios en tu zona local.`}
      searchText={row=>`${names.get(row.device_id)} ${row.device_id} ${row.app} ${row.title}`} columns={[
        {label:'Equipo',render:row=><Link to={`/devices/${encodeURIComponent(row.device_id)}?tab=activity`}>{names.get(row.device_id)||row.device_id}</Link>},
        {label:'Reporte',render:row=>row.is_stale?'Sin señal reciente':'Reciente'},
        {label:'Aplicación',render:row=>row.app||'Sin informar'},{label:'Ventana',render:row=>row.title||'Sin título'},
        {label:'Última señal',render:row=>new Date(row.last_seen).toLocaleString()}]} />
  </AppShell>;
}
