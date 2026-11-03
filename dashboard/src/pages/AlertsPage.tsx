import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { Icon } from '../components/Icon';
import { apiClient } from '../api/client';
import type { Device, SecurityAlert } from '../types';
const labels: Record<string, string> = { CRITICAL: 'Crítica', HIGH: 'Alta', MEDIUM: 'Media', LOW: 'Baja' };
const order: Record<string, number> = { CRITICAL: 0, HIGH: 1, MEDIUM: 2, LOW: 3 };
export function AlertsPage() {
  const [alerts, setAlerts] = useState<SecurityAlert[] | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [resolved, setResolved] = useState(false);
  const [severity, setSeverity] = useState('all');
  const [query, setQuery] = useState('');
  const [revision, setRevision] = useState(0);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<SecurityAlert | null>(null);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState('');
  const [notice, setNotice] = useState('');
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    let alive = true;
    setBusy(true); setError(''); setAlerts(null);
    Promise.allSettled([apiClient.getAlerts(undefined, resolved), apiClient.getDevices()] as const).then(([result, fleet]) => {
      if (!alive) return;
      if (result.status === 'fulfilled') setAlerts(result.value); else setError('No se pudieron consultar las alertas. Reintenta para conocer su estado.');
      if (fleet.status === 'fulfilled') setDevices(fleet.value);
      setBusy(false);
    });
    return () => { alive = false; };
  }, [resolved, revision]);
  useEffect(() => { if (selected) dialog.current?.showModal(); else dialog.current?.close(); }, [selected]);
  const names = new Map(devices.map(d => [d.device_id, d.nickname || d.hostname]));
  const visible = useMemo(() => (alerts ?? []).filter(a => (severity === 'all' || a.severity === severity) && `${a.alert_type} ${a.description} ${a.device_id} ${devices.find(d => d.device_id === a.device_id)?.hostname ?? ''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())).sort((a, b) => (order[a.severity] ?? 9) - (order[b.severity] ?? 9)), [alerts, severity, query, devices]);
  return <AppShell currentPage="alerts" title="Alertas" subtitle="Prioriza incidentes, consulta su contexto y registra su resolución." actions={<button className="button" disabled={busy} onClick={() => setRevision(v => v + 1)}><Icon name="refresh" />Actualizar</button>}>
    {notice && <p className="notice notice-success" role="status">{notice}</p>}{error && <p className="notice notice-error" role="alert">{error}</p>}
    <section className="panel"><div className="panel-header"><div><p className="eyebrow">Seguimiento de incidentes</p><h2>{resolved ? 'Alertas resueltas' : 'Alertas abiertas'}</h2></div><span className="muted">{alerts === null ? 'Sin consulta disponible' : `${visible.length} resultados`}</span></div>
      <div className="fleet-toolbar"><label className="search-field"><Icon name="search" /><span className="sr-only">Buscar alertas</span><input type="search" placeholder="Buscar tipo, descripción o equipo…" value={query} onChange={e => setQuery(e.target.value)} /></label><label className="sort-field">Severidad<select value={severity} onChange={e => setSeverity(e.target.value)}><option value="all">Todas</option>{Object.entries(labels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label></div>
      <div className="filter-tabs"><button aria-pressed={!resolved} className={!resolved ? 'active' : ''} onClick={() => setResolved(false)}>Abiertas</button><button aria-pressed={resolved} className={resolved ? 'active' : ''} onClick={() => setResolved(true)}>Resueltas</button></div>
      {busy ? <div className="empty-state" role="status">Consultando alertas…</div> : error ? <div className="empty-state"><Icon name="shield" /><h3>Estado no disponible</h3><button className="button" onClick={() => setRevision(v => v + 1)}>Reintentar</button></div> : visible.length === 0 ? <div className="empty-state"><Icon name="shield" /><h3>Sin alertas en esta vista</h3><p>{severity !== 'all' || query ? 'Cambia los filtros para ampliar la búsqueda.' : 'No hay registros para el estado seleccionado.'}</p></div> : <div className="table-scroll" role="region" aria-label="Alertas" tabIndex={0}><table><thead><tr><th scope="col">Severidad</th><th scope="col">Incidente</th><th scope="col">Equipo</th><th scope="col">Fecha local</th><th scope="col">Acción</th></tr></thead><tbody>{visible.map(alert => <tr key={alert.id}><td><span className={`severity severity-${alert.severity.toLowerCase()}`}>{labels[alert.severity] ?? alert.severity}</span></td><td><strong>{alert.alert_type.replaceAll('_', ' ')}</strong><p className="muted" style={{ maxWidth: 420 }}>{alert.description}</p>{alert.app_name && <span className="cell-secondary">{alert.app_name}</span>}</td><td><Link to={`/devices/${alert.device_id}`}>{names.get(alert.device_id) ?? alert.device_id.slice(0, 8)}</Link></td><td>{new Date(alert.created_at).toLocaleString()}</td><td>{alert.resolved ? <span className="status-badge online">Resuelta</span> : <button className="button button-small" onClick={() => { setSelected(alert); setActionError(''); }}>Resolver</button>}</td></tr>)}</tbody></table></div>}
    </section>
    <dialog ref={dialog} className="action-dialog" aria-labelledby="resolve-title" onCancel={e => { if (saving) e.preventDefault(); }} onClose={() => setSelected(null)}><header><h2 id="resolve-title">Resolver alerta</h2><button className="icon-button" aria-label="Cerrar" disabled={saving} onClick={() => setSelected(null)}><Icon name="close" /></button></header><p>{selected?.description}</p><p className="field-note" style={{ margin: '16px 0' }}>La alerta pasará al historial de resueltas. Confirma después de revisar el incidente.</p>{actionError && <p className="notice notice-error" role="alert">{actionError}</p>}<button className="button button-primary" disabled={saving} onClick={async () => { if (!selected || saving) return; setSaving(true); setActionError(''); try { await apiClient.resolveAlert(selected.id); setNotice(`Alerta #${selected.id} marcada como resuelta.`); setSelected(null); setRevision(v => v + 1); } catch { setActionError('No se pudo confirmar la resolución. Actualiza la lista antes de repetirla.'); } finally { setSaving(false); } }}>{saving ? 'Guardando…' : 'Confirmar resolución'}</button></dialog>
  </AppShell>;
}
