import { Link } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { CollectionTable } from '../components/CollectionTable';
import { Icon } from '../components/Icon';
import { useCollection } from '../hooks/useCollection';
import { apiClient } from '../api/client';
const load = () => apiClient.getUsbHistory(undefined, 1000);
export function USBPage() {
  const data = useCollection(load);
  return <AppShell currentPage="usb" title="Dispositivos USB" subtitle="Consulta las conexiones y desconexiones reportadas por los equipos." actions={<button className="button" disabled={data.loading} onClick={data.refresh}><Icon name="refresh" />Actualizar</button>}>
    <CollectionTable {...data} title="Historial USB" note="Hasta 1.000 eventos de la consulta. Fechas en la zona local del navegador. La conexión corresponde al momento del evento." searchText={event => `${event.device_name} ${event.device_id} ${event.serial_number} ${event.action} ${event.hardware_id}`} columns={[
      { label: 'Fecha local', render: event => new Date(event.timestamp).toLocaleString() },
      { label: 'Equipo', render: event => <Link to={`/devices/${event.device_id}`} title={event.device_id}>{event.device_id.slice(0,8)}…</Link> },
      { label: 'Dispositivo', render: event => <strong>{event.device_name || 'Sin nombre'}</strong> },
      { label: 'Número de serie', render: event => event.serial_number || 'No informado' },
      { label: 'Evento', render: event => <span className={`status-badge ${event.action === 'IN' ? 'online' : 'offline'}`}>{event.action === 'IN' ? 'Conexión' : 'Desconexión'}</span> },
    ]} />
  </AppShell>;
}
