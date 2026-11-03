import { useEffect, useRef, useState } from 'react';
import { apiClient } from '../api/client';
import type { Investigation } from '../api/client';
import type { Device } from '../types';

function ReleaseDialog({ item, done, close }: {item: Investigation; done: () => void; close: () => void}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => { dialog.current?.showModal(); }, []);
  return <dialog ref={dialog} className="action-dialog" aria-labelledby="release-title" onClose={close} onCancel={e => {if (busy) e.preventDefault();}}>
    <h2 id="release-title">Liberar conservación: {item.title}</h2>
    <p>Los eventos podrán eliminarse durante el mantenimiento si superan el plazo configurado y no están protegidos por otra investigación. Esta acción no elimina eventos inmediatamente.</p>
    <form onSubmit={async e => {e.preventDefault(); if (busy) return; setBusy(true); setError(''); try {await apiClient.releaseInvestigation(item.id, reason.trim()); done();} catch {setError('No se pudo confirmar la liberación. Actualiza el listado antes de repetirla.');} finally {setBusy(false);}}}>
      <label htmlFor="release-reason">Motivo de cierre</label><textarea id="release-reason" required maxLength={4000} value={reason} onChange={e => setReason(e.target.value)} />
      {error && <p role="alert" className="notice notice-error">{error}</p>}
      <div className="flex gap-2"><button className="button button-danger" disabled={busy || !reason.trim()}>Liberar protección</button><button type="button" className="button" disabled={busy} onClick={close}>Cancelar</button></div>
    </form>
  </dialog>;
}
export function InvestigationsPanel() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [deviceError, setDeviceError] = useState(false);
  const [device, setDevice] = useState('');
  const [filter, setFilter] = useState('');
  const [offset, setOffset] = useState(0);
  const [revision, setRevision] = useState(0);
  const [data, setData] = useState<{items: Investigation[]; has_more: boolean} | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [release, setRelease] = useState<Investigation | null>(null);
  useEffect(() => {let active=true; apiClient.getDevices().then(d => {if(active) {setDevices(d); setDeviceError(false);}}).catch(() => {if(active) setDeviceError(true);}); return () => {active=false;};}, [revision]);
  useEffect(() => {
    const abort=new AbortController();
    apiClient.getInvestigations(filter, offset, abort.signal).then(result => {if(!abort.signal.aborted) setData(result);}).catch(() => {if(!abort.signal.aborted) setError('No se pudieron consultar las investigaciones.');}).finally(() => {if(!abort.signal.aborted) setLoading(false);});
    return () => abort.abort();
  }, [filter, offset, revision]);
  const refresh=() => {setLoading(true); setError(''); setData(null); setRevision(v => v+1);};
  return <section className="panel mt-4 p-4" aria-labelledby="investigations-heading">
    <div className="flex flex-wrap items-center justify-between gap-2"><h2 id="investigations-heading">Investigaciones y conservación</h2><button className="button" disabled={busy} onClick={refresh}>Actualizar investigaciones</button></div>
    <p className="muted">Protege los eventos de seguridad de un equipo durante un período. Los datos ya eliminados no se recuperan. El período se indica en hora local y se guarda en UTC.</p>
    <details><summary>Abrir investigación</summary>
      {deviceError && <p className="notice notice-error" role="alert">No se pudieron cargar los equipos. Utiliza Actualizar investigaciones.</p>}
      <form className="grid gap-3 mt-3" onSubmit={async e => {
        e.preventDefault(); if(busy) return;
        const form=e.currentTarget; const values=new FormData(form);
        const from=new Date(String(values.get('from'))); const to=new Date(String(values.get('to')));
        if(!Number.isFinite(from.getTime()) || !Number.isFinite(to.getTime()) || to<from) {setMessage('El período debe tener fechas válidas y ordenadas.'); return;}
        setBusy(true); setMessage('');
        try {await apiClient.createInvestigation({device_id:device,title:String(values.get('title')).trim(),reason:String(values.get('reason')).trim(),event_from:from.toISOString(),event_to:to.toISOString()}); form.reset(); setDevice(''); setMessage('Investigación abierta. Protección registrada.'); refresh();}
        catch {setMessage('No se pudo confirmar la apertura. Actualiza el listado antes de volver a enviarla.');} finally {setBusy(false);}
      }}>
        <label>Equipo<select required value={device} onChange={e => setDevice(e.target.value)} disabled={busy}><option value="">Selecciona un equipo</option>{devices.map(d => <option key={d.device_id} value={d.device_id}>{d.nickname || d.hostname}</option>)}</select></label>
        <label>Título<input name="title" required maxLength={200} disabled={busy} /></label>
        <label>Motivo<textarea name="reason" required maxLength={4000} disabled={busy} /></label>
        <div className="grid gap-3 sm:grid-cols-2"><label>Desde (hora local)<input type="datetime-local" name="from" required disabled={busy} /></label><label>Hasta (hora local)<input type="datetime-local" name="to" required disabled={busy} /></label></div>
        <button className="button button-primary" disabled={busy || !device}>Abrir y proteger eventos</button>
      </form>
    </details>
    {message && <p className="notice" role="status">{message}</p>}
    <label>Filtrar investigaciones por equipo<select value={filter} disabled={busy} onChange={e => {setFilter(e.target.value); setOffset(0); refresh();}}><option value="">Todos los equipos</option>{devices.map(d => <option key={d.device_id} value={d.device_id}>{d.nickname || d.hostname}</option>)}</select></label>
    {error && <p className="notice notice-error" role="alert">{error}</p>}
    {loading ? <p role="status">Cargando investigaciones…</p> : data && <>
      {!data.items.length && <p>No hay investigaciones para este filtro.</p>}
      <div className="grid gap-3 mt-3">{data.items.map(item => <article className="rounded-lg border p-4" key={item.id}>
        <p className="muted">Abierta por {item.created_by || 'operador no registrado'}</p><h3>{item.title} · {item.released_at ? 'Liberada' : 'Protección activa'}</h3>
        <p>{devices.find(d => d.device_id===item.device_id)?.hostname || item.device_id}</p>
        <p>{new Date(item.event_from).toLocaleString()} — {new Date(item.event_to).toLocaleString()}</p><p className="whitespace-pre-wrap">{item.reason}</p>
        {item.released_at ? <p>Liberada por {item.released_by || 'operador no registrado'} el {new Date(item.released_at).toLocaleString()}: {item.release_reason}</p> : <button className="button" onClick={() => setRelease(item)}>Cerrar investigación y liberar</button>}
      </article>)}</div>
      <div className="flex items-center gap-3 mt-3"><button className="button" disabled={offset===0} onClick={() => {setOffset(v => Math.max(0,v-20)); refresh();}}>Anterior</button><span>Página {offset/20+1}</span><button className="button" disabled={!data.has_more} onClick={() => {setOffset(v => v+20); refresh();}}>Siguiente</button></div>
    </>}
    {release && <ReleaseDialog item={release} close={() => setRelease(null)} done={() => {setRelease(null); refresh();}} />}
  </section>;
}
