import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  Switch,
  FormControlLabel,
  Divider,
  Chip,
  List,
  ListItem,
  ListItemText,
} from '@mui/material';
import {
  Router,
  Speed,
  Settings,
  CheckCircle,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface SystemStatusCardProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

interface ProxyInfo {
  name: string;
  type: string;
  delay: number | null;
  alive: boolean;
}

const SystemStatusCard: React.FC<SystemStatusCardProps> = React.memo(({ isRunning, showNotification }) => {
  const [tunMode, setTunMode] = useState(false);
  const [config, setConfig] = useState<any>(null);
  const [proxies, setProxies] = useState<ProxyInfo[]>([]);
  const [totalProxies, setTotalProxies] = useState(0);
  const [onlineProxies, setOnlineProxies] = useState(0);

  useEffect(() => {
    loadSystemStatus();
  }, [isRunning]);

  const loadSystemStatus = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;

    try {
      // 只加载配置，不加载代理数据（减少API调用）
      const configData = await invoke<any>('get_mihomo_config');
      setConfig(configData);
      setTunMode(configData.tun?.enable || false);

      // 延迟加载代理数据，避免阻塞页面渲染
      if (isRunning) {
        setTimeout(async () => {
          try {
            const proxiesData = await invoke<any>('get_proxies');
            if (proxiesData.proxies) {
              const allProxies: ProxyInfo[] = Object.entries(proxiesData.proxies)
                .filter(([name, proxy]: [string, any]) => {
                  const excludeTypes = ['Selector', 'URLTest', 'Fallback', 'LoadBalance', 'Relay'];
                  const excludeNames = ['DIRECT', 'REJECT', 'COMPATIBLE', 'PASS', 'REJECT-DROP'];
                  return !excludeTypes.includes(proxy.type) && !excludeNames.includes(name);
                })
                .map(([name, proxy]: [string, any]) => ({
                  name,
                  type: proxy.type,
                  delay: proxy.history?.[0]?.delay || null,
                  alive: proxy.alive === true,
                }));
              
              // 统计所有节点
              setTotalProxies(allProxies.length);
              setOnlineProxies(allProxies.filter(p => p.alive).length);
              
              // 只显示前5个节点
              setProxies(allProxies.slice(0, 5));
            }
          } catch (error) {
            console.error('Failed to load proxies:', error);
          }
        }, 500); // 延迟500ms加载
      }
    } catch (error) {
      console.error('Failed to load system status:', error);
    }
  };

  const handleTunToggle = async (enable: boolean) => {
    try {
      await invoke<string>('enable_tun_mode', { enable });
      setTunMode(enable);
      showNotification(`TUN mode ${enable ? 'enabled' : 'disabled'}`, 'success');
    } catch (error) {
      showNotification(`Failed to ${enable ? 'enable' : 'disable'} TUN mode: ${error}`, 'error');
    }
  };

  const formatDelay = (delay: number | null): string => {
    if (!delay) return 'N/A';
    if (delay < 0) return 'Timeout';
    return `${delay}ms`;
  };

  const getDelayColor = (delay: number | null): 'success' | 'warning' | 'error' | 'default' => {
    if (!delay || delay < 0) return 'default';
    if (delay < 100) return 'success';
    if (delay < 300) return 'warning';
    return 'error';
  };

  return (
    <Card>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          系统状态
        </Typography>

        {/* Configuration Info */}
        <Box sx={{ mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
            <Settings fontSize="small" color="action" />
            <Typography variant="subtitle2" color="text.secondary">
              配置信息
            </Typography>
          </Box>
          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
            <Chip
              label={`端口: ${config?.port || 'N/A'}`}
              size="small"
              variant="outlined"
            />
            <Chip
              label={`Socks: ${config?.['socks-port'] || 'N/A'}`}
              size="small"
              variant="outlined"
            />
            <Chip
              label={`模式: ${config?.mode || 'N/A'}`}
              size="small"
              variant="outlined"
            />
            <Chip
              label={`DNS: ${config?.dns?.enable ? '已启用' : '已禁用'}`}
              size="small"
              variant="outlined"
              color={config?.dns?.enable ? 'success' : 'default'}
            />
          </Box>
        </Box>

        <Divider sx={{ my: 2 }} />

        {/* TUN Mode Toggle */}
        <Box sx={{ mb: 2 }}>
          <FormControlLabel
            control={
              <Switch
                checked={tunMode}
                onChange={(e) => handleTunToggle(e.target.checked)}
                disabled={!isRunning}
              />
            }
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Router fontSize="small" />
                <Typography variant="body2">
                  TUN 模式
                </Typography>
                {tunMode && (
                  <Chip label="已启用" size="small" color="success" />
                )}
              </Box>
            }
          />
        </Box>

        {/* Quick Stats */}
        {isRunning && (
          <>
            <Divider sx={{ my: 2 }} />
            <Box>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                <Speed fontSize="small" color="action" />
                <Typography variant="subtitle2" color="text.secondary">
                  快速统计
                </Typography>
              </Box>
              <Box sx={{ display: 'flex', gap: 2 }}>
                <Chip
                  label={`节点总数: ${totalProxies}`}
                  size="small"
                  variant="outlined"
                  color="primary"
                />
                <Chip
                  label={`在线: ${onlineProxies}`}
                  size="small"
                  variant="outlined"
                  color="success"
                />
              </Box>
            </Box>
          </>
        )}
      </CardContent>
    </Card>
  );
});

SystemStatusCard.displayName = 'SystemStatusCard';

export default SystemStatusCard;
