import { useState, useEffect } from 'react';

export interface ActivityItem {
  device_id: string;
  app: string;
  title: string;
  is_idle: boolean;
  is_live: boolean;
  last_seen: string;
}

export interface StreamData {
  activities: ActivityItem[];
  timestamp: string;
}

export function useActivityStream() {
  const [data, setData] = useState<StreamData | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let eventSource: EventSource | null = null;
    let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

    const connect = () => {
      try {
        eventSource = new EventSource('http://localhost:3000/api/stream');

        eventSource.onopen = () => {
          setIsConnected(true);
          setError(null);
        };

        eventSource.onmessage = (event) => {
          try {
            const parsed = JSON.parse(event.data);
            setData(parsed);
          } catch (err) {
            console.error('Error parsing stream data:', err);
          }
        };

        eventSource.onerror = () => {
          setIsConnected(false);
          setError('Connection lost. Reconnecting in 5 seconds...');
          eventSource?.close();
          
          // Reconnect after 5 seconds
          reconnectTimeout = setTimeout(() => {
            connect();
          }, 5000);
        };
      } catch (err: any) {
        setError(err.message || 'Failed to connect to activity stream');
        setIsConnected(false);
      }
    };

    connect();

    return () => {
      eventSource?.close();
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }
    };
  }, []);

  return { data, isConnected, error };
}
