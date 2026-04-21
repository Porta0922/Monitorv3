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
    let retryAttempt = 0;

    const connect = () => {
      try {
        eventSource = new EventSource(`http://${window.location.hostname}:3000/api/stream`);

        eventSource.onopen = () => {
          setIsConnected(true);
          setError(null);
          retryAttempt = 0;
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
          const nextDelay = Math.min(30000, 2000 * Math.pow(2, retryAttempt));
          retryAttempt += 1;
          setError(`Connection lost. Reconnecting in ${Math.round(nextDelay / 1000)} seconds...`);
          eventSource?.close();

          // Exponential backoff reconnect to reduce pressure when backend is unavailable
          reconnectTimeout = setTimeout(() => {
            connect();
          }, nextDelay);
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
