import {useCallback, useEffect} from 'react';
import {useRemoteSection} from '../../hooks/useRemoteSection';
import {CollectionTable} from '../CollectionTable';
import {SessionActivityPanel} from './SessionActivityPanel';
import {apiClient} from '../../api/client';
const number = (value: number | null | undefined, unit = '') =>
  value == null || !Number.isFinite(value) ? 'No disponible' : `${value.toLocaleString('es', { maximumFractionDigits: 1 })}${unit}`;
const bytes = (value: number | null | undefined) => value == null ? 'No disponible' : number(value / 1073741824, ' GiB');
const collectorStates: Record<string,string> = { unsupported_context: 'No disponible en este contexto', disabled: 'Desactivado por política', supervised: 'Supervisado', stopped: 'Detenido', not_started: 'Sin iniciar' };
const captureStates: Record<string,string> = { capture_ok: "Captura completada", capture_error: "Error de captura", no_observation: "Sin observación concluyente", unknown: "Calidad de captura sin informar" };
const names: Record<string, string> = { healthy: 'Operativo', degraded: 'Degradado', service: 'Servicio', session: 'Sesión', interactive: 'Interactivo' };

export function DeviceTelemetryPanel({deviceId, resourcesOnly=false}: {deviceId:string; resourcesOnly?:boolean}) {
  const {data:value,error,loading,refresh}=useRemoteSection(useCallback(()=>apiClient.getDeviceTelemetry(deviceId),[deviceId]));
  useEffect(()=>{if(loading)return;const timer=setInterval(()=>{if(!document.hidden)refresh();},30000);return()=>clearInterval(timer);},[loading,refresh]);
  const sample=value?.resources, metrics=sample?.data, health=value?.health;
  const caps=health?.data.capabilities??metrics?.capabilities;
  const state=!health?'Sin diagnóstico':health.stale?'Diagnóstico atrasado':health.data.monitoring_paused?'Recolección pausada':names[health.data.status??'']??'Estado desconocido';
  return <section className="panel" aria-label={resourcesOnly?'Recursos actuales':'Estado actual del agente'}>
    <div className="panel-header"><div><h2>{resourcesOnly?'Recursos actuales':'Salud del agente'}</h2><p className="muted">Última muestra, independiente de la fecha histórica seleccionada</p></div><button className="button" disabled={loading} onClick={refresh}>{loading?'Actualizando…':'Actualizar'}</button></div>
    <div className="p-5 space-y-4">
      {loading&&!value&&<p role="status">Consultando telemetría…</p>}
      {error&&<p role="alert" className="notice notice-warning">No se pudo actualizar. {value?'Se conserva la consulta anterior, que puede estar desactualizada.':'No hay datos confirmados.'} Usa Actualizar para reintentar.</p>}
      {!resourcesOnly&&<><p><strong>{state}</strong> · {caps?.os??'Sistema sin informar'} · {names[caps?.mode??'']??caps?.mode??'Modo sin informar'} · Versión {health?.data.version??'sin informar'}</p><p className="muted">Diagnóstico: {health?new Date(health.observed_at).toLocaleString():'sin recibir'}. Cola pendiente: {number(health?.data.cache_unsynced_events)} eventos.</p><p>Entrada: {caps?.input_available==null?'sin informar':caps.input_available?'reportada disponible':'no disponible en este contexto'}.</p>{caps?.input_diagnostic&&<details><summary>Adaptador de entrada</summary><p>{caps.input_diagnostic.state} · {caps.input_diagnostic.last_error??'Sin error informado'} · {caps.input_diagnostic.observed_at?new Date(caps.input_diagnostic.observed_at).toLocaleString():'Sin fecha'}</p></details>}</>}
      <p className={sample?.stale?'notice notice-warning':'muted'}>{sample?`${sample.stale?'Muestra atrasada':'Última muestra'}: ${new Date(sample.observed_at).toLocaleString()}`:'Sin muestra de recursos disponible.'}</p>
      <dl className="device-resource-values"><div><dt>CPU</dt><dd>{metrics?.cpu_state==='warming_up'?'Preparando lectura':number(metrics?.cpu_percent,' %')}</dd></div><div><dt>Memoria</dt><dd>{bytes(metrics?.memory_used_bytes)} / {bytes(metrics?.memory_total_bytes)} · {number(metrics?.memory_percent,' %')}</dd></div><div><dt>Encendido</dt><dd>{metrics?.uptime_seconds==null?'Sin informar':number(metrics.uptime_seconds/3600,' h')}</dd></div></dl>
      <p className="muted">{metrics?.hardware?.os_version??'Sistema sin informar'} · {metrics?.hardware?.cpu_model??'Procesador sin informar'} · {number(metrics?.hardware?.logical_cpu_count)} CPU lógicas.</p>
      <details><summary>Almacenamiento</summary><CollectionTable title="Volúmenes" rows={metrics?.disks??null} loading={loading} error={error} refresh={refresh} note="Capacidad informada por la última muestra del agente." searchText={d=>d.mount} columns={[{label:'Montaje',render:d=>d.mount},{label:'Capacidad',render:d=>bytes(d.total_bytes)},{label:'Disponible',render:d=>bytes(d.available_bytes)},{label:'Uso',render:d=>number(d.used_percent,' %')}]}/></details>
      <details><summary>Red por interfaz</summary><CollectionTable title="Interfaces de red" rows={metrics?.network??null} loading={loading} error={error} refresh={refresh} note="Promedio del intervalo; las interfaces virtuales pueden reflejar el mismo tráfico." searchText={n=>n.name} columns={[{label:'Interfaz',render:n=>n.name},{label:'Recepción',render:n=>n.receive_bytes_per_second==null?'Sin intervalo comparable':number(n.receive_bytes_per_second/1024,' KiB/s')},{label:'Envío',render:n=>n.transmit_bytes_per_second==null?'Sin intervalo comparable':number(n.transmit_bytes_per_second/1024,' KiB/s')}]}/></details>
      {!resourcesOnly&&<>
        <details><summary>Disponibilidad por recolector</summary><CollectionTable title="Recolectores" rows={health?.data.collectors??null} loading={loading} error={error} refresh={refresh} note="Supervisado no confirma acceso al escritorio. La observación puede preceder a una pausa o reinicio." searchText={c=>`${c.name} ${c.state}`} columns={[{label:'Recolector',render:c=>c.name},{label:'Estado',render:c=>collectorStates[c.state]??'Desconocido'},{label:'Captura',render:c=>captureStates[c.capture?.state??'unknown']??'Sin confirmar'},{label:'Observación',render:c=>c.capture?.observed_at?new Date(c.capture.observed_at).toLocaleString():'Sin fecha'},{label:'Detalle',render:c=>c.capture?.last_error??c.last_error??'Sin error informado'}]}/></details>
        <details><summary>Diagnóstico de sesiones de escritorio</summary>
          <p className="muted">Instancias observadas durante los últimos 15 minutos. No equivalen a usuarios conectados ni a sesiones RDP. La sesión informa cada 30 segundos; el servicio consolida cada cinco minutos.</p>
          {!health?.data.sessions?<p>El agente no informó diagnóstico de sesiones.</p>:!health.data.sessions.relay_enabled?<p>La conexión de sesiones no está habilitada.</p>:<>
            <p>{health.data.sessions.producer_mode==='per_principal'?'Credenciales independientes por persona.':'Productor heredado: una sesión a la vez.'}</p>
            {health.data.sessions.truncated&&<p className="notice notice-warning">Reporte truncado por el agente; no incluye todas las instancias.</p>}
            <CollectionTable title="Instancias de sesión" rows={health.data.sessions.instances} loading={loading} error={error} refresh={refresh} note="Una señal reciente no confirma que todas las capturas funcionen." searchText={s=>`${s.principal} ${s.instance_id}`} columns={[
              {label:'Persona / instancia',render:s=>`${s.principal??'Origen heredado'} / ${s.instance_id}`},
              {label:'Última señal',render:s=>new Date(s.received_at).toLocaleString()},
              {label:'Estado',render:s=>health.stale||s.stale||Date.now()-Date.parse(s.received_at)>390000?'Diagnóstico atrasado':s.monitoring_paused?'Pausada':'Señal reciente'},
              {label:'Entrada',render:s=>s.input_available?'Reportada disponible':'No disponible'},
              {label:'Pendientes',render:s=>number(s.pending_events)},
              {label:'Política',render:s=>health.stale||s.stale||Date.now()-Date.parse(s.received_at)>390000||!health.data.policy_fingerprint?'Sin confirmación reciente':s.policy_fingerprint===health.data.policy_fingerprint?'Coincide':'Diferente'},
              {label:'Tareas',render:s=><details><summary>Ver tareas</summary><ul>{s.tasks.map(t=><li key={t.name}>{t.name}: {t.running?'supervisada':'detenida'} · {captureStates[t.capture?.state??'unknown']??'Sin diagnóstico'} · {t.capture?.last_error??'Sin error informado'}</li>)}</ul></details>}]}/>
          </>}
        </details>
        <SessionActivityPanel deviceId={deviceId} compact/>
        <details><summary>Diagnóstico de tareas</summary><CollectionTable title="Tareas del agente" rows={health?.data.tasks??null} loading={loading} error={error} refresh={refresh} note="Una tarea supervisada puede estar esperando una política; no confirma acceso al sistema." searchText={t=>t.name} columns={[{label:'Tarea',render:t=>t.name},{label:'Estado',render:t=>t.running?'Supervisada':'Detenida'},{label:'Reinicios',render:t=>t.restart_count},{label:'Detalle',render:t=>t.last_error??'Sin error informado'}]}/></details>
      </>}
    </div>
  </section>;
}
