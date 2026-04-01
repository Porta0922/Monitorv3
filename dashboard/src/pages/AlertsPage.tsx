import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';
import type { SecurityAlert } from '../types';

export function AlertsPage() {
  const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadAlerts();
  }, []);

  const loadAlerts = async () => {
    try {
      setIsLoading(true);
      const data = await apiClient.getAlerts(undefined, false);
      setAlerts(data);
    } catch (err) {
      console.error('Error loading alerts:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleResolveAlert = async (alertId: number) => {
    try {
      await apiClient.resolveAlert(alertId);
      loadAlerts();
    } catch (err) {
      console.error('Error resolving alert:', err);
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'CRITICAL':
        return '#c33';
      case 'HIGH':
        return '#f80';
      case 'MEDIUM':
        return '#fc0';
      case 'LOW':
        return '#0066cc';
      default:
        return '#666';
    }
  };

  const getAlertIcon = (alertType: string) => {
    switch (alertType) {
      case 'PROCESS_TERMINATION_ATTEMPTED':
        return '⚠️ ';
      case 'HASH_MISMATCH':
        return '🔒 ';
      case 'UNAUTHORIZED_ACCESS':
        return '🚫 ';
      default:
        return '🔔 ';
    }
  };

  // Filter for critical process termination alerts
  const criticalAlerts = alerts.filter(a => a.alert_type === 'PROCESS_TERMINATION_ATTEMPTED');


  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="alerts" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        {/* Critical Alerts Section - PROCESS TERMINATION */}
        {criticalAlerts.length > 0 && (
          <div style={{
            backgroundColor: '#c33',
            color: 'white',
            padding: '1.5rem',
            borderRadius: '8px',
            marginBottom: '2rem',
            boxShadow: '0 4px 12px rgba(204, 51, 51, 0.3)'
          }}>
            <h3 style={{ margin: '0 0 0.5rem 0', fontSize: '1.2rem' }}>
              ⚠️ CRITICAL: Process Termination Attempts Detected
            </h3>
            <p style={{ margin: '0 0 1rem 0', fontSize: '0.95rem', opacity: 0.9 }}>
              {criticalAlerts.length} agent(s) have reported termination attempts. 
              The agent(s) have been protected and automatically restarted.
            </p>
            <div style={{ display: 'grid', gap: '0.5rem' }}>
              {criticalAlerts.map((alert) => (
                <div key={alert.id} style={{
                  backgroundColor: 'rgba(255,255,255,0.1)',
                  padding: '1rem',
                  borderRadius: '4px',
                  borderLeft: '3px solid white'
                }}>
                  <p style={{ margin: '0 0 0.5rem 0', fontWeight: 'bold' }}>
                    {alert.alert_type}
                  </p>
                  <p style={{ margin: 0, fontSize: '0.9rem' }}>
                    {alert.description}
                  </p>
                  <p style={{ margin: '0.5rem 0 0 0', fontSize: '0.85rem', opacity: 0.8 }}>
                    🕐 {new Date(alert.created_at).toLocaleString()}
                  </p>
                </div>
              ))}
            </div>
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <h2 style={{ margin: 0 }}>Security Alerts ({alerts.length})</h2>
          <button
            onClick={loadAlerts}
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
            Loading security alerts...
          </div>
        ) : alerts.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            ✓ No security alerts
          </div>
        ) : (
          <div style={{ display: 'grid', gap: '1rem' }}>
            {alerts.map((alert) => (
              <div
                key={alert.id}
                style={{
                  backgroundColor: 'white',
                  borderRadius: '8px',
                  padding: '1.5rem',
                  borderLeft: `4px solid ${getSeverityColor(alert.severity)}`,
                  boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
                  opacity: alert.alert_type === 'PROCESS_TERMINATION_ATTEMPTED' ? 1 : 1
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start', marginBottom: '1rem' }}>
                  <div>
                    <h3 style={{ margin: '0 0 0.5rem 0', color: '#333' }}>
                      {getAlertIcon(alert.alert_type)} {alert.alert_type}
                    </h3>
                    <p style={{ margin: 0, fontSize: '0.85rem', color: '#666' }}>
                      {alert.app_name ? `Application: ${alert.app_name}` : 'System Alert'}
                    </p>
                  </div>
                  <span style={{
                    padding: '0.25rem 0.75rem',
                    borderRadius: '20px',
                    fontSize: '0.85rem',
                    fontWeight: '500',
                    backgroundColor: getSeverityColor(alert.severity) + '20',
                    color: getSeverityColor(alert.severity)
                  }}>
                    {alert.severity}
                  </span>
                </div>

                <p style={{ margin: '0.5rem 0', fontSize: '0.9rem', color: '#555' }}>
                  {alert.description}
                </p>

                <div style={{ fontSize: '0.85rem', color: '#666', marginBottom: '1rem' }}>
                  <p style={{ margin: '0.25rem 0' }}>🕐 Created: {new Date(alert.created_at).toLocaleString()}</p>
                  {alert.exe_hash && (
                    <p style={{ margin: '0.25rem 0', fontFamily: 'monospace' }}>Hash: {alert.exe_hash.slice(0, 32)}...</p>
                  )}
                </div>

                <button
                  onClick={() => handleResolveAlert(alert.id)}
                  style={{
                    padding: '0.5rem 1rem',
                    backgroundColor: '#0066cc',
                    color: 'white',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontSize: '0.85rem'
                  }}
                >
                  ✓ Mark as Resolved
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
