import { Link } from 'react-router-dom';
import { AppShell } from '../components/AppShell';
import { CollectionTable } from '../components/CollectionTable';
import { Icon } from '../components/Icon';
import { useCollection } from '../hooks/useCollection';
import { apiClient } from '../api/client';
const load = () => apiClient.getWifiHistory(undefined, 200);
export function WifiPage() {
  const data = useCollection(load);
  return <AppShell currentPage="wifi" title="Redes WiFi" subtitle="Historial de conectividad inalámbrica reportado por los agentes." actions={<button className="button" disabled={data.loading} onClick={data.refresh}><Icon name="refresh" />Actualizar</button>}>
    <CollectionTable {...data} title="Eventos de conectividad" note="Hasta 200 eventos de la consulta. La señal y el estado corresponden a la fecha del evento, no a una medición en vivo." searchText={event => `${event.device_id} ${event.ssid ?? ''} ${event.interface_name} ${event.state}`} columns={[
      { label: 'Fecha local', render: event => new Date(event.timestamp).toLocaleString() },
      { label: 'Equipo', render: event => <Link to={`/devices/${event.device_id}`} title={event.device_id}>{event.device_id.slice(0,8)}…</Link> },
      { label: 'Red', render: event => <strong>{event.ssid || 'SSID no informado'}</strong> },
      { label: 'Interfaz', render: event => event.interface_name || 'No informada' },
      { label: 'Estado reportado', render: event => event.state || 'No informado' },
      { label: 'Señal', render: event => event.signal_percent == null ? 'No disponible' : `${event.signal_percent}%` },
    ]} />
  </AppShell>;
}
