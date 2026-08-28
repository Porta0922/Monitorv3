import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import type { WifiEvent } from '../types';

export function WifiPage() {
  const [events, setEvents] = useState<WifiEvent[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiClient.getWifiHistory(undefined, 200)
      .then(setEvents)
      .catch(() => setEvents([]))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div style={{ padding: 24 }}>Cargando...</div>;

  return (
    <div style={{ padding: 24 }}>
      <h1 style={{ fontSize: 24, fontWeight: 600, marginBottom: 16 }}>Historial WiFi</h1>
      {events.length === 0 ? (
        <p style={{ color: '#6b7280' }}>No hay eventos WiFi registrados.</p>
      ) : (
        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
          <thead>
            <tr style={{ borderBottom: '2px solid #e5e7eb' }}>
              <th style={{ textAlign: 'left', padding: 8 }}>Fecha</th>
              <th style={{ textAlign: 'left', padding: 8 }}>Dispositivo</th>
              <th style={{ textAlign: 'left', padding: 8 }}>SSID</th>
              <th style={{ textAlign: 'left', padding: 8 }}>Estado</th>
              <th style={{ textAlign: 'left', padding: 8 }}>Senal</th>
            </tr>
          </thead>
          <tbody>
            {events.map((e, i) => (
              <tr key={i} style={{ borderBottom: '1px solid #f3f4f6' }}>
                <td style={{ padding: 8 }}>{new Date(e.timestamp).toLocaleString()}</td>
                <td style={{ padding: 8 }}>{e.device_id.slice(0, 8)}</td>
                <td style={{ padding: 8 }}>{e.ssid || '-'}</td>
                <td style={{ padding: 8 }}>{e.state}</td>
                <td style={{ padding: 8 }}>{e.signal_percent != null ? `${e.signal_percent}%` : '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
