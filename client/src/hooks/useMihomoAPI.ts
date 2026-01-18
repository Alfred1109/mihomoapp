import { useState, useEffect, useCallback } from 'react';
import axios from 'axios';

interface MihomoStatus {
  isRunning: boolean;
  processId: number | null;
  apiStatus: any;
  mihomoPath?: string;
  configPath?: string;
}

interface MihomoConfig {
  port?: number;
  'socks-port'?: number;
  'allow-lan'?: boolean;
  mode?: string;
  'log-level'?: string;
  tun?: {
    enable: boolean;
    stack?: string;
    'auto-route'?: boolean;
  };
  [key: string]: any;
}

interface ProxyInfo {
  [key: string]: {
    type: string;
    name: string;
    now?: string;
    all?: string[];
    history?: any[];
  };
}

export const useMihomoAPI = () => {
  const [status, setStatus] = useState<MihomoStatus | null>(null);
  const [config, setConfig] = useState<MihomoConfig | null>(null);
  const [proxies, setProxies] = useState<ProxyInfo | null>(null);
  const [traffic, setTraffic] = useState<any>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const apiCall = useCallback(async (endpoint: string, options: any = {}) => {
    try {
      setError(null);
      const response = await axios({
        url: `/api/mihomo${endpoint}`,
        method: options.method || 'GET',
        data: options.data,
        ...options
      });
      return response.data;
    } catch (err: any) {
      const errorMessage = err.response?.data?.error || err.message || 'Unknown error';
      setError(errorMessage);
      throw new Error(errorMessage);
    }
  }, []);

  // Fetch mihomo status
  const fetchStatus = useCallback(async () => {
    try {
      const statusData = await apiCall('/status');
      setStatus(statusData);
      return statusData;
    } catch (error) {
      console.error('Failed to fetch status:', error);
    }
  }, [apiCall]);

  // Fetch mihomo config
  const fetchConfig = useCallback(async () => {
    try {
      const configData = await apiCall('/config');
      setConfig(configData);
      return configData;
    } catch (error) {
      console.error('Failed to fetch config:', error);
    }
  }, [apiCall]);

  // Fetch proxies
  const fetchProxies = useCallback(async () => {
    try {
      const proxiesData = await apiCall('/proxies');
      setProxies(proxiesData.proxies);
      return proxiesData.proxies;
    } catch (error) {
      console.error('Failed to fetch proxies:', error);
    }
  }, [apiCall]);

  // Fetch traffic stats
  const fetchTraffic = useCallback(async () => {
    try {
      const trafficData = await apiCall('/traffic');
      setTraffic(trafficData);
      return trafficData;
    } catch (error) {
      console.error('Failed to fetch traffic:', error);
    }
  }, [apiCall]);

  // Start mihomo service
  const startMihomo = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await apiCall('/start', { method: 'POST' });
      await fetchStatus();
      return result;
    } finally {
      setIsLoading(false);
    }
  }, [apiCall, fetchStatus]);

  // Stop mihomo service
  const stopMihomo = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await apiCall('/stop', { method: 'POST' });
      await fetchStatus();
      return result;
    } finally {
      setIsLoading(false);
    }
  }, [apiCall, fetchStatus]);

  // Restart mihomo service
  const restartMihomo = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await apiCall('/restart', { method: 'POST' });
      await fetchStatus();
      return result;
    } finally {
      setIsLoading(false);
    }
  }, [apiCall, fetchStatus]);

  // Switch proxy for a group
  const switchProxy = useCallback(async (groupName: string, proxyName: string) => {
    try {
      const result = await apiCall(`/proxies/${groupName}`, {
        method: 'PUT',
        data: { name: proxyName }
      });
      await fetchProxies();
      return result;
    } catch (error) {
      console.error('Failed to switch proxy:', error);
      throw error;
    }
  }, [apiCall, fetchProxies]);

  // Update config
  const updateConfig = useCallback(async (configUpdates: Partial<MihomoConfig>) => {
    try {
      const result = await apiCall('/config', {
        method: 'PATCH',
        data: configUpdates
      });
      await fetchConfig();
      return result;
    } catch (error) {
      console.error('Failed to update config:', error);
      throw error;
    }
  }, [apiCall, fetchConfig]);

  // Toggle TUN mode
  const toggleTun = useCallback(async (enabled: boolean) => {
    try {
      const action = enabled ? 'enable' : 'disable';
      const result = await apiCall(`/tun/${action}`, { method: 'POST' });
      await fetchConfig();
      return result;
    } catch (error) {
      console.error('Failed to toggle TUN mode:', error);
      throw error;
    }
  }, [apiCall, fetchConfig]);

  // Initialize data on mount
  useEffect(() => {
    const initializeData = async () => {
      await Promise.all([
        fetchStatus(),
        fetchConfig()
      ]);
    };

    initializeData();
  }, [fetchStatus, fetchConfig]);

  // Refresh data periodically when running
  useEffect(() => {
    if (!status?.isRunning) return;

    const interval = setInterval(async () => {
      await Promise.all([
        fetchStatus(),
        fetchTraffic()
      ]);
    }, 5000); // Update every 5 seconds

    return () => clearInterval(interval);
  }, [status?.isRunning, fetchStatus, fetchTraffic]);

  return {
    // State
    status,
    config,
    proxies,
    traffic,
    isLoading,
    error,

    // Actions
    startMihomo,
    stopMihomo,
    restartMihomo,
    switchProxy,
    updateConfig,
    toggleTun,

    // Fetch methods
    fetchStatus,
    fetchConfig,
    fetchProxies,
    fetchTraffic
  };
};
