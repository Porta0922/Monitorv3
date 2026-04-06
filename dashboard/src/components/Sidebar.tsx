import { useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { apiClient } from '../api/client';

interface NavItem {
  path: string;
  label: string;
  icon: string;
  current: string;
}

export function Sidebar({ currentPage }: { currentPage: string }) {
  const navigate = useNavigate();
  const [newAlertsCount, setNewAlertsCount] = useState(0);

  const handleLogout = () => {
    apiClient.logout();
    navigate('/login');
  };

  const navItems: NavItem[] = [
    { path: '/dashboard', label: 'Overview', icon: '◆', current: 'dashboard' },
    { path: '/activity', label: 'En Vivo', icon: '◉', current: 'activity' },
    { path: '/alerts', label: 'Historial', icon: '◌', current: 'alerts' },
  ];

  useEffect(() => {
    let isMounted = true;

    const loadAlerts = async () => {
      try {
        const alerts = await apiClient.getAlerts(undefined, false);
        if (isMounted) {
          setNewAlertsCount(alerts.length);
        }
      } catch {
        if (isMounted) {
          setNewAlertsCount(0);
        }
      }
    };

    loadAlerts();
    const interval = setInterval(loadAlerts, 15000);

    return () => {
      isMounted = false;
      clearInterval(interval);
    };
  }, []);

  return (
    <div className="sticky top-0 h-screen w-[220px] border-r border-[#16213f] bg-[#060a1a] flex flex-col">
      {/* Header */}
      <div className="h-16 border-b border-[#16213f] px-5 flex items-center">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-display text-xl font-black tracking-widest text-[#00d9ff]">AME</p>
            {newAlertsCount > 0 && <span className="inline-block h-2 w-2 rounded-full bg-red-500"></span>}
          </div>
          <p className="font-mono text-[10px] tracking-[0.2em] text-[#6f7ea8]">ENTERPRISE</p>
        </div>
      </div>

      {/* Navigation Items */}
      <nav className="flex-1 pt-4 px-2 space-y-1">
        {navItems.map((item) => (
          <button
            key={item.path}
            onClick={() => navigate(item.path)}
            className={`w-full flex items-center gap-3 px-4 py-2 rounded-md transition-all duration-200 ${
              currentPage === item.current
                ? 'bg-[#032538] text-[#00d9ff] border border-[#00d9ff]/30'
                : 'text-[#8a97ba] hover:text-[#dbe4ff] hover:bg-[#0d1630]'
            }`}
          >
            <span className="text-xs w-4 flex-shrink-0">{item.icon}</span>
            <span className="font-mono text-xs tracking-wide">{item.label}</span>
            {item.current === 'alerts' && newAlertsCount > 0 && (
              <span className="ml-auto inline-flex items-center gap-1 rounded-full border border-red-500/40 bg-red-500/15 px-2 py-[2px] font-mono text-[9px] text-red-300">
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-red-400"></span>
                {newAlertsCount}
              </span>
            )}
          </button>
        ))}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-[#1e2339]">
        <button
          onClick={handleLogout}
          className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-[#111c3b] hover:bg-[#163059] text-[#8a97ba] hover:text-[#00d9ff] rounded-md transition-all text-xs"
        >
          <span>◌</span>
          Logout
        </button>
      </div>
    </div>
  );
}
