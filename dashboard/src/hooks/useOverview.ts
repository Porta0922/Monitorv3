import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';

export interface OverviewData {
  devices_today: number;
  active_time: number;
  idle_time: number;
  idle_pct: string;
  keys_today: number;
  mouse_moves_today: number;
  mouse_clicks_today: number;
}

export function useOverview() {
  const [data, setData] = useState<OverviewData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchOverview = async () => {
      try {
        setIsLoading(true);
        const result = await apiClient.getOverview();
        setData(result);
        setError(null);
      } catch (err: any) {
        setError(err.message || 'Failed to load overview');
        console.error('Error loading overview:', err);
      } finally {
        setIsLoading(false);
      }
    };

    fetchOverview();
    // Refresh every 30 seconds
    const interval = setInterval(fetchOverview, 30000);
    return () => clearInterval(interval);
  }, []);

  return { data, isLoading, error };
}
