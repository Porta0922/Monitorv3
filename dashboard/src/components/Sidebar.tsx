import {apiClient} from '../api/client';
import { Link } from 'react-router-dom';
import { Icon } from './Icon';
import type { IconName } from './Icon';
const groups: { label: string; items: [string, string, string, IconName][] }[] = [
  { label: 'Supervisión', items: [['dashboard', '/dashboard', 'Equipos', 'grid'], ['activity', '/activity', 'Actividad', 'activity'], ['metrics', '/metrics', 'Métricas', 'chart'], ['administration', '/administration', 'Administración', 'monitor'], ['session', '/session', 'Sesiones y acceso', 'shield']] },
  { label: 'Protección', items: [['alerts', '/alerts', 'Alertas', 'bell'], ['security', '/security', 'Eventos de seguridad', 'shield']] },
  { label: 'Equipos y conexiones', items: [['inventory', '/inventory', 'Inventario', 'box'], ['usb', '/usb', 'Dispositivos USB', 'usb'], ['wifi', '/wifi', 'Redes WiFi', 'wifi']] },
];
export function Sidebar({ currentPage, onNavigate }: { currentPage: string; onNavigate?: () => void }) {
  return <aside className="workspace-sidebar">
    <Link className="brand" to="/dashboard" onClick={onNavigate}><span className="brand-mark"><Icon name="activity" /></span><span>Monitor<span className="brand-sub">Administración de equipos</span></span></Link>
    <nav aria-label="Navegación principal">{groups.map(group => <div className="nav-group" key={group.label}>
      <p className="nav-label">{group.label}</p>{group.items.filter(([key])=>key!=='administration'||apiClient.getRole()==='admin').map(([key, path, label, icon]) => <Link key={key} to={path} onClick={onNavigate} aria-current={currentPage === key ? 'page' : undefined} className={`nav-link ${currentPage === key ? 'selected' : ''}`}><Icon name={icon} /><span>{label}</span></Link>)}
    </div>)}</nav>
    <div className="sidebar-note"><span className="eyebrow">Espacio de trabajo</span><strong>Operaciones de TI</strong><p>Visibilidad de tu infraestructura, equipo por equipo.</p></div>
  </aside>;
}
