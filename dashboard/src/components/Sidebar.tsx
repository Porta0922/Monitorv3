import { useNavigate } from 'react-router-dom';
import { useState } from 'react';
import { apiClient } from '../api/client';

interface NavItem {
  path: string;
  label: string;
  icon: string;
  current: string;
}

export function Sidebar({ currentPage }: { currentPage: string }) {
  const navigate = useNavigate();
  const [isExpanded, setIsExpanded] = useState(true);

  const handleLogout = () => {
    apiClient.logout();
    navigate('/login');
  };

  const navItems: NavItem[] = [
    { path: '/dashboard', label: 'Dashboard', icon: '📊', current: 'dashboard' },
    { path: '/activity', label: 'Activity', icon: '📈', current: 'activity' },
    { path: '/inventory', label: 'Inventory', icon: '📦', current: 'inventory' },
    { path: '/usb', label: 'USB', icon: '💾', current: 'usb' },
    { path: '/alerts', label: 'Alerts', icon: '🚨', current: 'alerts' },
  ];

  return (
    <div
      className={`sticky top-0 h-screen bg-[#131829] border-r border-[#1e2339] transition-all duration-300 flex flex-col ${
        isExpanded ? 'w-64' : 'w-20'
      }`}
    >
      {/* Header */}
      <div className="flex items-center justify-between h-20 px-4 border-b border-[#1e2339]">
        {isExpanded && (
          <div className="flex items-center gap-2">
            <span className="text-[#00d9ff] text-2xl font-bold">●</span>
            <span className="text-[#e4e6eb] font-bold text-sm">Activity</span>
          </div>
        )}
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="p-1 hover:bg-[#1a1f3a] rounded transition-colors text-[#a0a5b2] hover:text-[#00d9ff]"
        >
          {isExpanded ? '◀' : '▶'}
        </button>
      </div>

      {/* Navigation Items */}
      <nav className="flex-1 pt-8 px-2 space-y-2">
        {navItems.map((item) => (
          <button
            key={item.path}
            onClick={() => navigate(item.path)}
            className={`w-full flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200 ${
              currentPage === item.current
                ? 'bg-gradient-to-r from-[#00d9ff]/20 to-[#00ff88]/20 text-[#00d9ff] border border-[#00d9ff]/50'
                : 'text-[#a0a5b2] hover:text-[#e4e6eb] hover:bg-[#1a1f3a]'
            }`}
          >
            <span className="text-xl w-6 flex-shrink-0">{item.icon}</span>
            {isExpanded && <span className="text-sm font-medium">{item.label}</span>}
          </button>
        ))}
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-[#1e2339]">
        <button
          onClick={handleLogout}
          className={`w-full flex items-center justify-center gap-2 px-4 py-2 bg-[#1a1f3a] hover:bg-[#00d9ff]/10 text-[#a0a5b2] hover:text-[#00d9ff] rounded-lg transition-all text-sm`}
        >
          <span>🚪</span>
          {isExpanded && 'Logout'}
        </button>
      </div>
    </div>
  );
}
