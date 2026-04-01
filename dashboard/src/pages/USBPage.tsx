import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';
import type { USBEvent } from '../types';

export function USBPage() {
  const navigate = useNavigate();
  const [events, setEvents] = useState<USBEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadUSBHistory();
  }, []);

  const loadUSBHistory = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getUsbHistory(undefined, 1000);
      setEvents(data);
    } catch (err) {
      console.error('Error loading USB history:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="usb" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <h2 style={{ margin: 0 }}>USB Device Events ({events.length})</h2>
          <button
            onClick={loadUSBHistory}
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

        {isLoading ? (
          <div style={{ textAlign: 'center', padding: '2rem', color: '#666' }}>
            Loading USB events...
          </div>
        ) : events.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            No USB events recorded yet
          </div>
        ) : (
          <div style={{ backgroundColor: 'white', borderRadius: '8px', overflow: 'hidden', boxShadow: '0 2px 8px rgba(0,0,0,0.1)' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.9rem' }}>
              <thead>
                <tr style={{ backgroundColor: '#f5f5f5', borderBottom: '2px solid #ddd' }}>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Timestamp</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Device</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Device Name</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Serial</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Action</th>
                </tr>
              </thead>
              <tbody>
                {events.slice(0, 100).map((event, idx) => (
                  <tr key={idx} style={{ borderBottom: '1px solid #eee' }}>
                    <td style={{ padding: '1rem', color: '#666', fontSize: '0.85rem' }}>
                      {new Date(event.timestamp).toLocaleString()}
                    </td>
                    <td style={{ padding: '1rem', color: '#666', fontFamily: 'monospace', fontSize: '0.8rem' }}>
                      {event.device_id.slice(0, 8)}...
                    </td>
                    <td style={{ padding: '1rem', color: '#333', fontWeight: '500' }}>
                      {event.device_name}
                    </td>
                    <td style={{ padding: '1rem', color: '#666', fontFamily: 'monospace', fontSize: '0.85rem' }}>
                      {event.serial_number}
                    </td>
                    <td style={{ padding: '1rem' }}>
                      <span style={{
                        padding: '0.25rem 0.75rem',
                        borderRadius: '20px',
                        fontSize: '0.85rem',
                        fontWeight: '500',
                        backgroundColor: event.action === 'IN' ? '#efe' : '#fee',
                        color: event.action === 'IN' ? '#060' : '#c33'
                      }}>
                        {event.action === 'IN' ? '🔌 Connected' : '🔌 Disconnected'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        <button
          onClick={() => navigate('/dashboard')}
          style={{
            marginTop: '1rem',
            padding: '0.5rem 1rem',
            backgroundColor: '#666',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer'
          }}
        >
          ← Back to Dashboard
        </button>
      </div>
    </div>
  );
}
