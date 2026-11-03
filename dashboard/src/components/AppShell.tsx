import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { ThemeSelect } from './ThemeSelect';
import { Sidebar } from './Sidebar';
import { Icon } from './Icon';
interface AppShellProps { currentPage: string; title: string; subtitle?: string; actions?: ReactNode; children: ReactNode; noScroll?: boolean; contentClassName?: string }
export function AppShell({ currentPage, title, subtitle, actions, children, contentClassName = '' }: AppShellProps) {
  const navigate = useNavigate();
  const menu = useRef<HTMLDialogElement>(null);
  const [logoutError,setLogoutError]=useState('');
  const [menuOpen, setMenuOpen] = useState(false);
  useEffect(() => { document.title = `${title} · Monitor`; }, [title]);
  useEffect(() => { if (menuOpen) menu.current?.showModal(); else menu.current?.close(); }, [menuOpen]);
  return <div className="workspace">
    <a href="#main-content" className="skip-link">Saltar al contenido</a>
    <div className="desktop-navigation"><Sidebar currentPage={currentPage} /></div>
    <dialog ref={menu} className="mobile-navigation" onClose={() => setMenuOpen(false)} onCancel={() => setMenuOpen(false)} aria-label="Menú de navegación">
      <button className="icon-button mobile-close" aria-label="Cerrar menú" onClick={() => setMenuOpen(false)}><Icon name="close" /></button><Sidebar currentPage={currentPage} onNavigate={() => setMenuOpen(false)} />
    </dialog>
    <div className="workspace-main">
      <div className="workspace-topbar"><div className="topbar-context"><button className="icon-button mobile-menu" aria-label="Abrir navegación" onClick={() => setMenuOpen(true)}><Icon name="menu" /></button><span>Monitor</span><span aria-hidden="true">/</span><strong>Consola de administración</strong></div><div className="topbar-actions"><ThemeSelect /><button className="button button-quiet" onClick={async () => {try{await apiClient.logout();navigate('/login');}catch{setLogoutError('No se pudo confirmar el cierre de sesión. Reintenta cuando vuelva la conexión.');}}}><Icon name="logout" />Cerrar sesión</button></div></div>
      <main id="main-content" tabIndex={-1} className={`workspace-content ${contentClassName}`}>
        <header className="page-header"><div><p className="eyebrow">Supervisión de equipos de oficina</p><h1>{title}</h1>{subtitle && <p className="page-description">{subtitle}</p>}</div>{actions && <div className="page-actions">{actions}</div>}</header>
        <div className="page-body">{logoutError&&<p role="alert" className="notice notice-error">{logoutError}</p>}{children}</div>
        <footer className="workspace-footer">Monitor <span>Los horarios se muestran según la zona indicada en cada vista.</span></footer>
      </main>
    </div>
  </div>;
}
