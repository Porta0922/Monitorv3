import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import { NavBar } from '../components/NavBar';

interface Heatmap {
  timestamp: string;
  device_id: string;
  grid_data: Record<string, number>;
  screen_width: number;
  screen_height: number;
  stats: {
    mouse_moves: number;
    mouse_clicks: number;
    keyboard_events: number;
  };
}

export function HeatmapsPage() {
  const [heatmaps, setHeatmaps] = useState<Heatmap[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [selectedDevice, setSelectedDevice] = useState<string>('');

  useEffect(() => {
    loadHeatmaps();
  }, [selectedDevice]);

  const loadHeatmaps = async () => {
    try {
      setIsLoading(true);
      if (selectedDevice) {
        // TODO: Implement API call for device-specific heatmaps
        // const data = await apiClient.getDeviceHeatmaps(selectedDevice);
        // setHeatmaps(data);
      }
    } catch (err) {
      console.error('Error loading heatmaps:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const renderHeatmapGrid = (heatmap: Heatmap) => {
    const gridSize = 100;
    const cellSize = 300 / gridSize;  // 300px canvas for display

    return (
      <div style={{
        position: 'relative',
        width: '300px',
        height: '300px',
        backgroundColor: '#f0f0f0',
        borderRadius: '4px',
        overflow: 'hidden',
        border: '1px solid #ddd'
      }}>
        {/* Render heatmap cells */}
        {Object.entries(heatmap.grid_data).map(([key, count]) => {
          const [x, y] = key.split(',').map(Number);
          const intensity = Math.min(count / 10, 1);  // Normalize intensity

          return (
            <div
              key={key}
              style={{
                position: 'absolute',
                left: `${x * cellSize}px`,
                top: `${y * cellSize}px`,
                width: `${cellSize}px`,
                height: `${cellSize}px`,
                backgroundColor: `rgba(255, ${100 - intensity * 100}, 0, ${intensity * 0.7})`,
                border: '0.5px solid rgba(0,0,0,0.05)'
              }}
              title={`Clicks: ${count}`}
            />
          );
        })}

        {/* Legend */}
        <div style={{
          position: 'absolute',
          bottom: '5px',
          right: '5px',
          fontSize: '10px',
          backgroundColor: 'rgba(255,255,255,0.8)',
          padding: '3px 5px',
          borderRadius: '2px'
        }}>
          High ← → Low
        </div>
      </div>
    );
  };

  return (
    <div style={{ minHeight: '100vh', backgroundColor: '#f5f5f5' }}>
      <NavBar currentPage="heatmaps" />

      <div style={{ maxWidth: '1200px', margin: '0 auto', padding: '2rem' }}>
        <h2 style={{ marginBottom: '1.5rem' }}>📊 Keyboard/Mouse Activity Heatmaps</h2>

        <div style={{ marginBottom: '2rem' }}>
          <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: 'bold' }}>
            Select Device:
          </label>
          <select
            value={selectedDevice}
            onChange={(e) => setSelectedDevice(e.target.value)}
            style={{
              padding: '0.5rem',
              borderRadius: '4px',
              border: '1px solid #ddd',
              fontSize: '1rem'
            }}
          >
            <option value="">Choose a device...</option>
            {/* TODO: Load devices from API */}
          </select>
        </div>

        {isLoading ? (
          <div style={{ textAlign: 'center', padding: '2rem', color: '#666' }}>
            Loading heatmaps...
          </div>
        ) : heatmaps.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '2rem',
            backgroundColor: 'white',
            borderRadius: '8px',
            color: '#666'
          }}>
            No heatmap data available. Select a device to view activity.
          </div>
        ) : (
          <div style={{ display: 'grid', gap: '2rem' }}>
            {heatmaps.map((heatmap, idx) => (
              <div
                key={idx}
                style={{
                  backgroundColor: 'white',
                  borderRadius: '8px',
                  padding: '1.5rem',
                  boxShadow: '0 2px 8px rgba(0,0,0,0.1)'
                }}
              >
                <div style={{ display: 'flex', gap: '2rem' }}>
                  {/* Heatmap Visualization */}
                  <div>
                    <h3 style={{ margin: '0 0 1rem 0', fontSize: '0.95rem' }}>
                      Activity Map
                    </h3>
                    {renderHeatmapGrid(heatmap)}
                  </div>

                  {/* Statistics */}
                  <div style={{ flex: 1 }}>
                    <h3 style={{ margin: '0 0 1rem 0', fontSize: '0.95rem' }}>
                      Activity Statistics
                    </h3>
                    <div style={{ display: 'grid', gap: '0.5rem' }}>
                      <div style={{ padding: '0.75rem', backgroundColor: '#f9f9f9', borderRadius: '4px' }}>
                        <p style={{ margin: 0, color: '#666', fontSize: '0.85rem' }}>Mouse Movements</p>
                        <p style={{ margin: '0.25rem 0 0 0', fontSize: '1.5rem', fontWeight: 'bold', color: '#0066cc' }}>
                          {heatmap.stats.mouse_moves}
                        </p>
                      </div>
                      <div style={{ padding: '0.75rem', backgroundColor: '#f9f9f9', borderRadius: '4px' }}>
                        <p style={{ margin: 0, color: '#666', fontSize: '0.85rem' }}>Mouse Clicks</p>
                        <p style={{ margin: '0.25rem 0 0 0', fontSize: '1.5rem', fontWeight: 'bold', color: '#ff6600' }}>
                          {heatmap.stats.mouse_clicks}
                        </p>
                      </div>
                      <div style={{ padding: '0.75rem', backgroundColor: '#f9f9f9', borderRadius: '4px' }}>
                        <p style={{ margin: 0, color: '#666', fontSize: '0.85rem' }}>Keyboard Events</p>
                        <p style={{ margin: '0.25rem 0 0 0', fontSize: '1.5rem', fontWeight: 'bold', color: '#00cc66' }}>
                          {heatmap.stats.keyboard_events}
                        </p>
                      </div>
                    </div>

                    {/* Timestamp */}
                    <p style={{
                      margin: '1.5rem 0 0 0',
                      fontSize: '0.85rem',
                      color: '#999',
                      borderTop: '1px solid #eee',
                      paddingTop: '1rem'
                    }}>
                      📅 {new Date(heatmap.timestamp).toLocaleString()}
                    </p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
