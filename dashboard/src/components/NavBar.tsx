import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';

interface NavBarProps {
  currentPage?: string;
}

export function NavBar({ currentPage }: NavBarProps) {
  const navigate = useNavigate();

  const handleLogout = () => {
    apiClient.logout();
    navigate('/login');
  };

  const navStyle = {
    display: 'flex',
    gap: '1rem',
    alignItems: 'center',
  };

  const navButtonStyle = (isActive: boolean) => ({
    padding: '0.5rem 1rem',
    backgroundColor: isActive ? 'rgba(255,255,255,0.3)' : 'transparent',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '0.9rem',
    textDecoration: 'none',
    fontWeight: isActive ? '600' : '400',
  });

  return (
    <div style={{
      backgroundColor: '#0066cc',
      color: 'white',
      padding: '1rem 1.5rem',
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
    }}>
      <h1 style={{ margin: 0, fontSize: '1.3rem' }}>🎯 ActivityMonitor</h1>
      
      <div style={navStyle}>
        <button
          onClick={() => navigate('/dashboard')}
          style={navButtonStyle(currentPage === 'dashboard')}
        >
          📊 Dashboard
        </button>
        <button
          onClick={() => navigate('/activity')}
          style={navButtonStyle(currentPage === 'activity')}
        >
          📈 Activity
        </button>
        <button
          onClick={() => navigate('/inventory')}
          style={navButtonStyle(currentPage === 'inventory')}
        >
          📦 Inventory
        </button>
        <button
          onClick={() => navigate('/usb')}
          style={navButtonStyle(currentPage === 'usb')}
        >
          🔌 USB
        </button>
        <button
          onClick={() => navigate('/alerts')}
          style={navButtonStyle(currentPage === 'alerts')}
        >
          🚨 Alerts
        </button>
      </div>

      <button
        onClick={handleLogout}
        style={{
          padding: '0.5rem 1rem',
          backgroundColor: 'rgba(255,255,255,0.2)',
          color: 'white',
          border: '1px solid white',
          borderRadius: '4px',
          cursor: 'pointer',
          fontSize: '0.85rem',
        }}
      >
        Logout
      </button>
    </div>
  );
}
