import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';
import type { Device } from '../types';

export function DashboardPage() {
  const navigate = useNavigate();
  const [devices, setDevices] = useState<Device[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    loadDevices();
  }, []);

  const loadDevices = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getDevices();
      setDevices(data);
    } catch (err: any) {
      setError(err.message || 'Failed to load devices');
      console.error('Error loading devices:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleUpdateNickname = async (deviceId: string, currentNickname?: string) => {
    const nickname = prompt('Enter device nickname:', currentNickname || '');
    if (nickname !== null) {
      try {
        await apiClient.updateDevice(deviceId, nickname);
        loadDevices();
      } catch (err) {
        alert('Failed to update nickname');
      }
    }
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="dashboard" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <h2 style={{ margin: 0 }}>Monitored Devices ({devices.length})</h2>
          <button
            onClick={loadDevices}
            style={{
              padding: '0.5rem 1rem',
              backgroundColor: '#0066cc',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer'
            }}
          >
            🔄 Refresh
          </button>
        </div>

        {error && (
          <div style={{
            padding: '1rem',
            backgroundColor: '#fee',
            color: '#c33',
            borderRadius: '4px',
            marginBottom: '1rem'
          }}>
            {error}
          </div>
        )}

        {isLoading ? (
          <div style={{ textAlign: 'center', padding: '2rem', color: '#666' }}>
            Loading devices...
          </div>
        ) : devices.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            <p>No devices registered yet</p>
            <p style={{ fontSize: '0.9rem' }}>Devices will appear here once agents connect</p>
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(350px, 1fr))', gap: '1.5rem' }}>
            {devices.map((device) => (
              <div
                key={device.device_id}
                style={{
                  backgroundColor: 'white',
                  borderRadius: '8px',
                  padding: '1.5rem',
                  boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
                  cursor: 'pointer',
                  transition: 'transform 0.2s, box-shadow 0.2s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.transform = 'translateY(-4px)';
                  e.currentTarget.style.boxShadow = '0 4px 16px rgba(0,0,0,0.15)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.transform = 'translateY(0)';
                  e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.1)';
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start', marginBottom: '1rem' }}>
                  <div>
                    <h3 style={{ margin: '0 0 0.5rem 0', color: '#333' }}>
                      {device.nickname || device.hostname}
                    </h3>
                    <p style={{ margin: 0, fontSize: '0.85rem', color: '#666' }}>
                      {device.hostname}
                    </p>
                  </div>
                  <span style={{
                    padding: '0.25rem 0.75rem',
                    borderRadius: '20px',
                    fontSize: '0.85rem',
                    fontWeight: '500',
                    backgroundColor: device.online ? '#efe' : '#fee',
                    color: device.online ? '#060' : '#c33'
                  }}>
                    {device.online ? '🟢 Online' : '🔴 Offline'}
                  </span>
                </div>

                <div style={{ fontSize: '0.85rem', color: '#666', marginBottom: '1rem' }}>
                  <p style={{ margin: '0.25rem 0' }}>📍 MAC: {device.mac_address}</p>
                  <p style={{ margin: '0.25rem 0' }}>⏱️ Last seen: {new Date(device.last_seen).toLocaleString()}</p>
                </div>

                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleUpdateNickname(device.device_id, device.nickname);
                    }}
                    style={{
                      flex: 1,
                      padding: '0.5rem',
                      backgroundColor: '#f0f0f0',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer',
                      fontSize: '0.85rem'
                    }}
                  >
                    ✎ Edit
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      navigate(`/device/${device.device_id}`);
                    }}
                    style={{
                      flex: 1,
                      padding: '0.5rem',
                      backgroundColor: '#0066cc',
                      color: 'white',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer',
                      fontSize: '0.85rem'
                    }}
                  >
                    View Details
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
