import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';
import type { ActivityLog } from '../types';

export function ActivityPage() {
  const navigate = useNavigate();
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadActivityLogs();
  }, []);

  const loadActivityLogs = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getActivityLogs(undefined, 1000);
      setLogs(data);
    } catch (err) {
      console.error('Error loading activity logs:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="activity" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <h2 style={{ margin: 0 }}>Recent Activity ({logs.length})</h2>
          <button
            onClick={loadActivityLogs}
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
            Loading activity logs...
          </div>
        ) : logs.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            No activity recorded yet
          </div>
        ) : (
          <div style={{ backgroundColor: 'white', borderRadius: '8px', overflow: 'hidden', boxShadow: '0 2px 8px rgba(0,0,0,0.1)' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.9rem' }}>
              <thead>
                <tr style={{ backgroundColor: '#f5f5f5', borderBottom: '2px solid #ddd' }}>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Timestamp</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Device</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Application</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Window Title</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Duration</th>
                </tr>
              </thead>
              <tbody>
                {logs.slice(0, 100).map((log, idx) => (
                  <tr key={idx} style={{ borderBottom: '1px solid #eee' }}>
                    <td style={{ padding: '1rem', color: '#666' }}>
                      {new Date(log.timestamp).toLocaleString()}
                    </td>
                    <td style={{ padding: '1rem', color: '#666', fontFamily: 'monospace', fontSize: '0.85rem' }}>
                      {log.device_id.slice(0, 8)}...
                    </td>
                    <td style={{ padding: '1rem', color: '#333', fontWeight: '500' }}>
                      {log.app_name}
                    </td>
                    <td style={{ padding: '1rem', color: '#666', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {log.window_title}
                    </td>
                    <td style={{ padding: '1rem', color: '#666' }}>
                      {log.duration_seconds}s
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
