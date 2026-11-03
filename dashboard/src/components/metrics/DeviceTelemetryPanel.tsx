import { useEffect, useState } from 'react';
import {SessionActivityPanel} from './SessionActivityPanel';
import { apiClient } from '../../api/client';
import type { DeviceTelemetry } from '../../types';

const number = (value: number | null | undefined, unit = '') =>
  value == null || !Number.isFinite(value) ? 'No disponible' : `${value.toLocaleString('es', { maximumFractionDigits: 1 })}${unit}`;
const bytes = (value: number | null | undefined) => value == null ? 'No disponible' : number(value / 1073741824, ' GiB');
const collectorStates: Record<string,string> = { unsupported_context: 'No disponible en este contexto', disabled: 'Desactivado por política', supervised: 'Supervisado', stopped: 'Detenido', not_started: 'Sin iniciar' };
const captureStates: Record<string,string> = { capture_ok: "Captura completada", capture_error: "Error de captura", no_observation: "Sin observación concluyente", unknown: "Calidad de captura sin informar" };
const names: Record<string, string> = { healthy: 'Operativo', degraded: 'Degradado', service: 'Servicio', session: 'Sesión', interactive: 'Interactivo' };

export function DeviceTelemetryPanel({ deviceId, resourcesOnly=false }: { deviceId: string; resourcesOnly?: boolean }) {
  const [value, setValue] = useState<DeviceTelemetry | null>(null);
  const [error, setError] = useState(false);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    let busy = false;
    setValue(null); setError(false);
    const load = async () => {
      if (busy) return;
      busy = true;
      try {
        const next = await apiClient.getDeviceTelemetry(deviceId, controller.signal);
        if (!controller.signal.aborted) { setValue(next); setError(false); }
      } catch { if (!controller.signal.aborted) setError(true); }
      finally { busy = false; }
    };
    void load();
    const timer = setInterval(() => { if (!document.hidden) void load(); }, 30000);
    return () => { controller.abort(); clearInterval(timer); };
  }, [deviceId, revision]);
  const sample = value?.resources;
  const metrics = sample?.data;
  const health = value?.health;
  const caps = health?.data.capabilities ?? metrics?.capabilities;
  const state = !health ? 'Sin diagnóstico' : health.stale ? 'Diagnóstico atrasado' : health.data.monitoring_paused ? 'Monitoreo pausado' : names[health.data.status ?? ''] ?? 'Estado desconocido';
  if(resourcesOnly) return <section className="panel"><div className="panel-header"><div><h2>Recursos actuales</h2><p className="muted">{sample?`${sample.stale?'Muestra atrasada':'Última muestra'}: ${new Date(sample.observed_at).toLocaleString()}`:'Sin muestra disponible'}</p></div><button className="button" onClick={()=>setRevision(v=>v+1)}>Actualizar</button></div><div className="p-5 space-y-4">{error&&<p role="alert">No se pudo actualizar; los datos pueden estar desactualizados.</p>}<dl className="device-resource-values"><div><dt>CPU</dt><dd>{number(metrics?.cpu_percent,' %')}</dd></div><div><dt>Memoria</dt><dd>{bytes(metrics?.memory_used_bytes)} / {bytes(metrics?.memory_total_bytes)}</dd></div><div><dt>Encendido</dt><dd>{metrics?.uptime_seconds==null?'Sin informar':number(metrics.uptime_seconds/3600,' h')}</dd></div></dl><details className="device-disclosure"><summary>Almacenamiento · {metrics?.disks?.length??0} volúmenes</summary><div className="table-scroll"><table><thead><tr><th>Unidad</th><th>Capacidad</th><th>Disponible</th><th>Uso</th></tr></thead><tbody>{metrics?.disks?.map((d,i)=><tr key={i}><td>{d.mount}</td><td>{bytes(d.total_bytes)}</td><td>{bytes(d.available_bytes)}</td><td>{number(d.used_percent,' %')}</td></tr>)}</tbody></table></div></details><details className="device-disclosure"><summary>Red · {metrics?.network?.length??0} interfaces</summary><p className="muted">Las interfaces virtuales pueden reflejar el mismo tráfico.</p><div className="table-scroll"><table><thead><tr><th>Interfaz</th><th>Recepción</th><th>Envío</th></tr></thead><tbody>{metrics?.network?.map(n=><tr key={n.name}><td>{n.name}</td><td>{n.receive_bytes_per_second==null?'Sin intervalo':number(n.receive_bytes_per_second/1024,' KiB/s')}</td><td>{n.transmit_bytes_per_second==null?'Sin intervalo':number(n.transmit_bytes_per_second/1024,' KiB/s')}</td></tr>)}</tbody></table></div></details></div></section>;
  return <section className="panel" aria-label="Estado actual del agente">
    <div className="panel-header"><div><p className="eyebrow">Telemetría actual</p><h2>Recursos y salud del agente</h2></div><button className="button" onClick={() => setRevision(v => v + 1)}>Actualizar</button></div>
    <div className="p-5 space-y-4">
      {error && <p role="alert" className="notice notice-warning">No se pudo actualizar. Los valores anteriores pueden estar desactualizados.</p>}
      {!value && !error && <p role="status">Consultando telemetría…</p>}
      {value && <>
        <p><strong>{state}</strong> · {caps?.os ?? 'Sistema sin informar'} · {names[caps?.mode ?? ''] ?? caps?.mode ?? 'Modo sin informar'} · Versión {health?.data.version ?? 'sin informar'}</p>
        <p className="muted">Entrada de usuario: {caps?.input_available == null ? 'sin informar' : caps.input_available ? 'disponible' : 'no disponible en este contexto'}. Cola pendiente: {number(health?.data.cache_unsynced_events)} eventos.</p>
        {caps?.input_diagnostic&&<p className="muted">Adaptador de entrada: {caps.input_diagnostic.state}{caps.input_diagnostic.last_error?` · ${caps.input_diagnostic.last_error}`:''}{caps.input_diagnostic.observed_at?` · ${new Date(caps.input_diagnostic.observed_at).toLocaleString()}`:''}</p>}
        <p className="muted">Diagnóstico: {health ? new Date(health.observed_at).toLocaleString() : 'aún no recibido'}.</p>
        {!sample ? <p className="notice">No hay métricas del nuevo contrato. El agente puede estar pausado, ser de una versión anterior o no haber enviado su primera muestra.</p> : <>
          <p className={sample.stale ? 'notice notice-warning' : 'muted'}>{sample.stale ? 'Última muestra atrasada' : 'Última muestra'}: {new Date(sample.observed_at).toLocaleString()}. Independiente de la fecha histórica seleccionada.</p>
          <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div><dt>CPU</dt><dd>{metrics?.cpu_state === 'warming_up' ? 'Preparando primera lectura' : number(metrics?.cpu_percent, ' %')}</dd></div>
            <div><dt>Memoria usada</dt><dd>{bytes(metrics?.memory_used_bytes)} / {bytes(metrics?.memory_total_bytes)}</dd></div>
            <div><dt>Uso de memoria</dt><dd>{number(metrics?.memory_percent, ' %')}</dd></div>
            <div><dt>Tiempo encendido</dt><dd>{metrics?.uptime_seconds == null ? 'No disponible' : number(metrics.uptime_seconds / 3600, ' h')}</dd></div>
          </dl>
          <p className="muted">{metrics?.hardware?.os_version ?? 'Versión del sistema sin informar'} · {metrics?.hardware?.cpu_model ?? 'Procesador sin informar'} · {number(metrics?.hardware?.logical_cpu_count)} CPU lógicas.</p>
          <h3>Volúmenes de almacenamiento</h3>
          {!metrics?.disks?.length ? <p>No disponibles en esta muestra.</p> : <div className="table-scroll"><table className="w-full text-left"><thead><tr><th>Montaje</th><th>Capacidad</th><th>Disponible</th><th>Uso</th></tr></thead><tbody>{metrics.disks.map((disk, i) => <tr key={`${disk.mount}-${i}`}><td>{disk.mount}</td><td>{bytes(disk.total_bytes)}</td><td>{bytes(disk.available_bytes)}</td><td>{number(disk.used_percent, ' %')}</td></tr>)}</tbody></table></div>}
          <h3>Red por interfaz</h3><p className="muted">Promedio del intervalo; las interfaces virtuales pueden reflejar el mismo tráfico.</p>
          {!metrics?.network?.length ? <p>No disponible en esta muestra.</p> : <div className="table-scroll"><table className="w-full text-left"><thead><tr><th>Interfaz</th><th>Recepción</th><th>Envío</th></tr></thead><tbody>{metrics.network.map(net => <tr key={net.name}><td>{net.name}</td><td>{net.receive_bytes_per_second == null ? 'Sin intervalo comparable' : number(net.receive_bytes_per_second / 1024, ' KiB/s')}</td><td>{net.transmit_bytes_per_second == null ? 'Sin intervalo comparable' : number(net.transmit_bytes_per_second / 1024, ' KiB/s')}</td></tr>)}</tbody></table></div>}
        </>}
        {!!health?.data.collectors?.length && <details><summary>Disponibilidad por recolector</summary><ul>{health.data.collectors.map(collector => <li key={collector.name}><strong>{collector.name}</strong>: {collectorStates[collector.state] ?? 'Estado desconocido'}{collector.last_error ? ` · ${collector.last_error}` : ''}<div className="muted">{captureStates[collector.capture?.state ?? 'unknown'] ?? 'Calidad desconocida'}{collector.capture?.last_error ? ` · ${collector.capture.last_error}` : ''}{collector.capture?.observed_at ? ` · Observado: ${new Date(collector.capture.observed_at).toLocaleString()}` : ''}</div></li>)}</ul><p className="muted">La captura muestra la última observación, que puede ser anterior a una pausa o reinicio. Supervisado no confirma acceso al sistema; sin observación no equivale a cero.</p></details>}
        <section aria-label="Sesiones de escritorio"><h3>Sesiones de escritorio</h3>
          {!health?.data.sessions?<p>Esta versión del agente no informó diagnóstico de sesiones.</p>:!health.data.sessions.relay_enabled?<p>La conexión de sesiones no está habilitada en el servicio.</p>:<>
            <p className="muted">Se muestran instancias de proceso observadas durante los últimos 15 minutos del servicio. No identifican usuarios ni equivalen a sesiones RDP. {health.data.sessions.producer_mode==='per_principal'?'Credenciales independientes por principal; actividad separada por ejecución.':'El relay heredado admite un productor a la vez.'}</p>
            {health.data.sessions.truncated&&<p className="notice notice-warning">Diagnóstico truncado por límite de tamaño; no se muestran todas las instancias.</p>}
            {!health.data.sessions.instances.length?<p>El servicio no informó instancias recientes. Esto no demuestra que no haya usuarios conectados.</p>:<div className="table-scroll"><table><thead><tr><th>Principal / instancia</th><th>Última señal al servicio</th><th>Estado</th><th>Entrada</th><th>Pendientes</th><th>Política respecto al servicio</th><th>Tareas</th></tr></thead><tbody>{health.data.sessions.instances.map(session=>{
              const stale=health.stale||session.stale||Date.now()-Date.parse(session.received_at)>390000;
              return <tr key={`${session.principal??'legacy'}-${session.instance_id}`}><td title={session.instance_id}>{session.principal??'Enrolamiento heredado'} / {session.instance_id.slice(0,8)}</td><td>{new Date(session.received_at).toLocaleString()}</td><td>{stale?'Diagnóstico atrasado':session.monitoring_paused?'Pausada':'Señal reciente'}</td><td>{session.input_available?'Reportada disponible':'No disponible'}</td><td>{number(session.pending_events)}</td><td>{stale||!health.data.policy_fingerprint?'Sin confirmación reciente':session.policy_fingerprint===health.data.policy_fingerprint?'Coincide':'Diferente'}</td><td><details><summary>Ver tareas</summary><ul>{session.tasks.map(task=><li key={task.name}>{task.name}: {task.running?'supervisada':'detenida'} · {captureStates[task.capture?.state??'unknown']??'Sin diagnóstico'}{task.capture?.last_error?` · ${task.capture.last_error}`:''}{task.capture?.observed_at?` · ${new Date(task.capture.observed_at).toLocaleString()}`:''}</li>)}</ul></details></td></tr>;
            })}</tbody></table></div>}
            <p className="muted">La sesión informa cada 30 segundos; el servicio consolida cada 5 minutos. Una señal reciente no confirma que todas las capturas funcionen.</p>
          </>}
        </section>
        <SessionActivityPanel deviceId={deviceId}/>
        <details><summary>Diagnóstico de tareas</summary>{!health?.data.tasks?.length ? <p>El agente no informó tareas.</p> : <ul>{health.data.tasks.map(task => <li key={task.name}><strong>{task.name}</strong>: {task.running ? 'supervisada' : 'detenida'} · {task.restart_count} reinicios{task.last_error ? ` · ${task.last_error}` : ''}</li>)}</ul>}<p>Una tarea supervisada puede estar esperando por una política; no confirma que su recolector tenga acceso al sistema.</p></details>
      </>}
    </div>
  </section>;
}
