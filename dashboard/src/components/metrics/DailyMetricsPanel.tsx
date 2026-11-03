import {useCallback} from 'react';
import {useRemoteSection} from '../../hooks/useRemoteSection';
import {useQueryState} from '../../hooks/useQueryState';
import {CollectionTable} from '../CollectionTable';
import {apiClient} from '../../api/client';
const sources={detail:'Muestras',summary:'Resumen diario',mixed:'Resumen + muestras'};
const percent=(value:number|null)=>value==null?'Sin datos':`${value.toFixed(2)} %`;
const hours=(value:number|null)=>value==null?'Sin datos':`${(value/3600).toFixed(2)} h`;
export function DailyMetricsPanel({deviceId,endDate}:{deviceId:string;endDate:string}) {
  const [rangeValue,setRange]=useQueryState('range','30',['historial-diario-page']);
  const range=[7,30,90,365].includes(Number(rangeValue))?Number(rangeValue):30;
  const {data,error,loading,refresh}=useRemoteSection(useCallback(()=>{
    const end=endDate>new Date().toISOString().slice(0,10)?new Date().toISOString().slice(0,10):endDate;
    const start=new Date(`${end}T00:00:00Z`);start.setUTCDate(start.getUTCDate()-range+1);
    return apiClient.getDailyMetrics(deviceId,start.toISOString().slice(0,10),end);
  },[deviceId,endDate,range]));
  return <section><label>Período del historial <select value={range} onChange={e=>setRange(e.target.value)}>{[7,30,90,365].map(n=><option key={n} value={n}>{n} días</option>)}</select></label>
    <CollectionTable title="Historial diario" rows={data?.days??null} loading={loading} error={error} refresh={refresh}
      note={`Días UTC. ${data?`${data.from} a ${data.to}. `:''}CPU y memoria: promedio por muestra. Actividad: duración acumulada. Detalle y resúmenes se combinan una sola vez. La calidad cuenta muestras, no horas; sin datos no significa cero.`}
      searchText={d=>d.day} columns={[
        {label:'Día UTC',render:d=>d.day},{label:'Origen',render:d=>sources[d.source]},
        {label:'Muestras',render:d=>d.resource_samples.toLocaleString('es')},
        {label:'CPU promedio / pico',render:d=>`${percent(d.cpu_average)} / ${percent(d.cpu_peak)}`},
        {label:'Memoria promedio / pico',render:d=>`${percent(d.memory_average)} / ${percent(d.memory_peak)}`},
        {label:'Actividad',render:d=>hours(d.active_seconds)},{label:'Inactividad',render:d=>hours(d.idle_seconds)},
        {label:'Calidad de entrada',render:d=>`${d.input_available_samples} disponibles · ${d.input_unavailable_samples} no disponibles · ${d.input_unknown_samples} sin confirmar`}]} />
  </section>;
}
