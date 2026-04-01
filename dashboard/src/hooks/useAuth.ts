// Authentication hook
import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';

export function useAuth() {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Check if already authenticated
    setIsAuthenticated(apiClient.isAuthenticated());
    setIsLoading(false);
  }, []);

  const login = async (username: string, password: string) => {
    try {
      await apiClient.login(username, password);
      setIsAuthenticated(true);
      return true;
    } catch (error) {
      console.error('Login failed:', error);
      return false;
    }
  };

  const logout = () => {
    apiClient.logout();
    setIsAuthenticated(false);
  };

  return { isAuthenticated, isLoading, login, logout };
}
