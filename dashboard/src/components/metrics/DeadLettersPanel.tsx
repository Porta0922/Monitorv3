import { useEffect, useState } from 'react';
import { apiClient } from '../../api/client';
import type { DeadLetterStatus } from '../../api/client';
export function DeadLettersPanel() {
  const [data,setData]=useState<DeadLetterStatus | null>(null);
  const [error,setError]=useState('');
  const [message,setMessage]=useState('');
  const [busy,setBusy]=useState(false);
  const [queue,setQueue]=useState('');
  const [reason,setReason]=useState('');
  const [limit,setLimit]=useState(10);
  const [revision,setRevision]=useState(0);
  useEffect(()=>{
    const abort=new AbortController();
    const load=()=>apiClient.getDeadLetters(abort.signal).then(d=>{if(!abort.signal.aborted){setData(d);setError('');}}).catch(()=>{if(!abort.signal.aborted)setError('No se pudo consultar RabbitMQ. Los conteos anteriores pueden estar desactualizados.');});
    void load(); const timer=setInterval(()=>void load(),60000);return ()=>{abort.abort();clearInterval(timer);};
  },[revision]);
  const pending=data?.queues.some(q=>(q.ready ?? 0)>0);
  return <details className="rounded-xl border p-4 text-sm"><summary>Mensajes pendientes de reparación{pending ? ' · Requiere atención' : ''}</summary>
    <p>Mensajes que no pudieron procesarse. Corrige primero la causa del fallo; luego reenvía un lote. El reenvío conserva el evento original y no confirma su procesamiento por el servidor.</p>
    {error && <p className="notice notice-error" role="alert">{error}</p>}
    {!data ? <p>Cargando colas…</p> : <><p>Consulta: {new Date(data.observed_at).toLocaleString()}. Conteos de mensajes listos; no incluyen entregas en curso.</p>
      <ul>{data.queues.map(q=><li key={q.queue}>{q.queue}: {q.available ? q.ready : 'No disponible'}</li>)}</ul>
      {data.queues.some(q=>!q.available) && <p role="alert">No se pudo consultar alguna cola; no se interpreta como vacía.</p>}
      <form className="grid gap-3 mt-3" onSubmit={async e=>{e.preventDefault();if(busy)return;setBusy(true);setMessage('');try{const r=await apiClient.replayDeadLetters(queue,limit,reason.trim());setMessage(`${r.acknowledged} mensajes devueltos a su cola de origen.${r.result.blocked ? ' El siguiente mensaje no es elegible; permanece protegido en la cola.' : ''}${!r.audit_completion_recorded ? ' No se pudo registrar el cierre en auditoría.' : ''}`);}catch{setMessage('No se pudo confirmar el resultado completo. Puede haber un lote parcial; actualiza los conteos y consulta la auditoría antes de repetir.');}finally{setBusy(false);setRevision(v=>v+1);}}}>
        <label>Cola<select required value={queue} disabled={busy} onChange={e=>setQueue(e.target.value)}><option value="">Selecciona una cola</option>{data.queues.map(q=><option key={q.queue} value={q.queue} disabled={!q.available || !q.ready}>{q.queue}</option>)}</select></label>
        <label>Máximo de mensajes<input type="number" min={1} max={20} required value={limit} disabled={busy} onChange={e=>setLimit(Number(e.target.value))}/></label>
        <label>Motivo y reparación realizada<textarea required maxLength={2000} value={reason} disabled={busy} onChange={e=>setReason(e.target.value)}/></label>
        <p>Ventana máxima: {data.replay_days} días. Un evento inválido o vencido detiene el lote y permanece en la cola. No se ofrece purga automática.</p>
        <button className="button button-primary" disabled={busy || !queue || !reason.trim()}>{busy ? 'Reenviando…' : 'Reenviar lote'}</button>
      </form></>}
    {message && <p className="notice" role="status">{message}</p>}
    <button className="button mt-3" disabled={busy} onClick={()=>setRevision(v=>v+1)}>Actualizar colas</button>
  </details>;
}
