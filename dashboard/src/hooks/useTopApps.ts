import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';

export interface AppData {
  app_name: string;
  total_duration_seconds: number;
  total_duration_hours: string;
}

export function useTopApps() {
  const [apps, setApps] = useState<AppData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchTopApps = async () => {
      try {
        setIsLoading(true);
        const result = await apiClient.getTopApps();
        setApps(result);
        setError(null);
      } catch (err: any) {
        setError(err.message || 'Failed to load top apps');
        console.error('Error loading top apps:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchTopApps();
    // Refresh every 60 seconds
    const interval = setInterval(fetchTopApps, 60000);
    return () => clearInterval(interval);
  }, []);

  return { apps, isLoading, error };
}
