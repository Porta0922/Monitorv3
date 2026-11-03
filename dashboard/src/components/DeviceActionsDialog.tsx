import { useEffect, useRef, useState } from 'react';
import { apiClient } from '../api/client';
import type { Device } from '../types';
import { Icon } from './Icon';
export function DeviceActionsDialog({ device, onClose, onChanged }: { device: Device; onClose: () => void; onChanged: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [nickname, setNickname] = useState(device.nickname ?? '');
  const [group,setGroup]=useState(device.group_name??'');
  const [version,setVersion]=useState(device.desired_version??'');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  useEffect(() => { dialog.current?.showModal(); }, []);
  const run = async () => {
    setBusy(true); setError(''); setMessage('');
    try {
      await apiClient.updateEquipment(device.device_id,nickname.trim(),group.trim(),version.trim());
      setMessage('Equipo actualizado. La versión objetivo no instala una actualización automáticamente.'); onChanged();
    } catch { setError('No se pudo confirmar la operación. Comprueba el estado del equipo antes de repetirla.'); }
    finally { setBusy(false); }
  };
  return <dialog ref={dialog} className="action-dialog" onCancel={event => { if (busy) event.preventDefault(); }} onClose={onClose} aria-labelledby="device-actions-title">
    <header><div><p className="eyebrow">Administrar equipo</p><h2 id="device-actions-title">{device.hostname}</h2></div><button className="icon-button" aria-label="Cerrar" disabled={busy} onClick={onClose}><Icon name="close" /></button></header>
    <form onSubmit={event => { event.preventDefault(); void run(); }}><label htmlFor="device-nickname">Nombre visible</label><input id="device-nickname" autoFocus maxLength={100} value={nickname} onChange={event => setNickname(event.target.value)} placeholder={device.hostname} /><p className="field-note">Un nombre que ayude a reconocer el equipo. Vacío conserva el hostname.</p><label htmlFor="device-group">Grupo / área de oficina</label><input id="device-group" maxLength={100} value={group} onChange={e=>setGroup(e.target.value)} placeholder="Ej.: Administración"/><label htmlFor="device-version">Versión objetivo</label><input id="device-version" maxLength={64} value={version} onChange={e=>setVersion(e.target.value)} placeholder="Sin objetivo"/><p className="field-note">La actualización se distribuye mediante los paquetes firmados. Guardar el objetivo no ejecuta cambios en el equipo.</p><button className="button button-primary" disabled={busy}>Guardar equipo</button><a className="text-link" href={`/administration?device=${encodeURIComponent(device.device_id)}`}>Política de este equipo</a></form>
    {message && <p className="notice notice-success" role="status">{message}</p>}{error && <p className="notice notice-error" role="alert">{error}</p>}
    <section className="notice"><h3>Retirar el agente</h3><p>La desinstalación requiere ejecutar el script administrativo en el equipo: uninstall-service.sh en Linux o uninstall-service.ps1 en Windows. Los scripts conservan identidad, configuración y datos pendientes. Las sesiones de usuario se administran por separado.</p></section>
  </dialog>;
}
