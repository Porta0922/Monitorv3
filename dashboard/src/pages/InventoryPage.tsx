import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';
import type { AppInfo } from '../types';

export function InventoryPage() {
  const navigate = useNavigate();
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSoftwareInventory();
  }, []);

  const loadSoftwareInventory = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getApps();
      setApps(data);
    } catch (err) {
      console.error('Error loading software inventory:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="inventory" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <h2 style={{ margin: 0 }}>Installed Applications ({apps.length})</h2>
          <button
            onClick={loadSoftwareInventory}
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
            Loading software inventory...
          </div>
        ) : apps.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            No software inventory available
          </div>
        ) : (
          <div style={{ backgroundColor: 'white', borderRadius: '8px', overflow: 'hidden', boxShadow: '0 2px 8px rgba(0,0,0,0.1)' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.9rem' }}>
              <thead>
                <tr style={{ backgroundColor: '#f5f5f5', borderBottom: '2px solid #ddd' }}>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Application</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Version</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>Status</th>
                  <th style={{ padding: '1rem', textAlign: 'left', fontWeight: '600', color: '#333' }}>SHA-256 Hash</th>
                </tr>
              </thead>
              <tbody>
                {apps.slice(0, 100).map((app, idx) => (
                  <tr key={idx} style={{ borderBottom: '1px solid #eee' }}>
                    <td style={{ padding: '1rem', color: '#333', fontWeight: '500' }}>
                      {app.app_name}
                    </td>
                    <td style={{ padding: '1rem', color: '#666' }}>
                      {app.version || 'Unknown'}
                    </td>
                    <td style={{ padding: '1rem' }}>
                      <span style={{
                        padding: '0.25rem 0.75rem',
                        borderRadius: '20px',
                        fontSize: '0.85rem',
                        fontWeight: '500',
                        backgroundColor: app.verified ? '#efe' : '#fee',
                        color: app.verified ? '#060' : '#c33'
                      }}>
                        {app.verified ? '✓ Verified' : '⚠ Unverified'}
                      </span>
                    </td>
                    <td style={{ padding: '1rem', color: '#666', fontFamily: 'monospace', fontSize: '0.8rem', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {app.exe_hash}
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
