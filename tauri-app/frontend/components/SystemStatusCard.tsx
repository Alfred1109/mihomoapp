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
  Alert,
  Button,
  CircularProgress,
} from '@mui/material';
import {
  Router,
  Speed,
  Settings,
  CheckCircle,
  AdminPanelSettings,
  Warning,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';
import { isTauriEnv } from '../utils/tauri';
import { useAppStore } from '../store/appStore';
import type { ConfigValue, ProxiesResponse, ProxyNode } from '../types';

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
  const { t } = useTranslation();
  const { isAdmin } = useAppStore();
  const [tunMode, setTunMode] = useState(false);
  const [config, setConfig] = useState<ConfigValue | null>(null);
  const [proxies, setProxies] = useState<ProxyInfo[]>([]);
  const [totalProxies, setTotalProxies] = useState(0);
  const [onlineProxies, setOnlineProxies] = useState(0);
  const [tunLoading, setTunLoading] = useState(false);

  useEffect(() => {
    loadSystemStatus();
  }, [isRunning]);

  const handleRestartAsAdmin = async () => {
    try {
      await invoke('restart_as_admin');
    } catch (error) {
      showNotification(`${t('systemStatus.restartAsAdminFailed')}: ${error}`, 'error');
    }
  };

  const loadSystemStatus = async () => {
    if (!isTauriEnv()) return;

    try {
      const configData = await invoke<ConfigValue>('get_mihomo_config');
      setConfig(configData);
      setTunMode(configData.tun?.enable || false);

      if (isRunning) {
        setTimeout(async () => {
          try {
            const proxiesData = await invoke<ProxiesResponse>('get_proxies');
            if (proxiesData.proxies) {
              const allProxies: ProxyInfo[] = Object.entries(proxiesData.proxies)
                .filter(([name, proxy]) => {
                  const excludeTypes = ['Selector', 'URLTest', 'Fallback', 'LoadBalance', 'Relay'];
                  const excludeNames = ['DIRECT', 'REJECT', 'COMPATIBLE', 'PASS', 'REJECT-DROP'];
                  return !excludeTypes.includes(proxy.type) && !excludeNames.includes(name);
                })
                .map(([name, proxy]) => {
                  const node = proxy as ProxyNode;
                  return {
                    name,
                    type: node.type,
                    delay: node.history?.[0]?.delay ?? null,
                    alive: node.alive === true,
                  };
                });
              
              setTotalProxies(allProxies.length);
              setOnlineProxies(allProxies.filter(p => p.alive).length);
              setProxies(allProxies.slice(0, 5));
            }
          } catch {
          }
        }, 500);
      }
    } catch {
    }
  };

  const handleTunToggle = async (enable: boolean) => {
    setTunLoading(true);
    try {
      if (enable && !isAdmin) {
        showNotification(t('systemStatus.tunRequiresAdmin'), 'error');
        setTunLoading(false);
        return;
      }

      await invoke<string>('enable_tun_mode', { enable });
      setTunMode(enable);
      
      if (isRunning) {
        showNotification(
          enable ? t('systemStatus.tunEnabling') : t('systemStatus.tunDisabling'),
          'info'
        );
        
        try {
          await invoke<string>('restart_mihomo_service_cmd');
          await new Promise(resolve => setTimeout(resolve, 2000));
          showNotification(
            enable ? t('systemStatus.tunEnabled') : t('systemStatus.tunDisabled'),
            'success'
          );
        } catch (restartError) {
          showNotification(
            `${t('systemStatus.tunSavedRestartFailed')}: ${restartError}`,
            'warning'
          );
        }
      } else {
        showNotification(
          enable ? t('systemStatus.tunEnabledStartService') : t('systemStatus.tunDisabledStartService'),
          'info'
        );
      }
    } catch (error) {
      showNotification(`${t('systemStatus.tunToggleFailed')}: ${error}`, 'error');
    } finally {
      setTunLoading(false);
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

        {/* Admin Status Warning */}
        {isAdmin === false && (
          <Alert 
            severity="warning" 
            sx={{ mb: 2 }}
            icon={<Warning />}
            action={
              <Button 
                color="inherit" 
                size="small"
                onClick={handleRestartAsAdmin}
                startIcon={<AdminPanelSettings />}
              >
                以管理员身份重启
              </Button>
            }
          >
            <Typography variant="body2">
              程序未以管理员身份运行，TUN模式将无法正常工作。
            </Typography>
          </Alert>
        )}

        {/* TUN Mode Toggle */}
        <Box sx={{ mb: 2 }}>
          <FormControlLabel
            control={
              tunLoading ? (
                <CircularProgress size={20} sx={{ mx: 1.5 }} />
              ) : (
                <Switch
                  checked={tunMode}
                  onChange={(e) => handleTunToggle(e.target.checked)}
                  disabled={!isRunning || tunLoading}
                />
              )
            }
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Router fontSize="small" />
                <Typography variant="body2">
                  TUN 模式（全局代理）
                </Typography>
                {tunMode && (
                  <Chip label="已启用" size="small" color="success" />
                )}
                {!isAdmin && tunMode && (
                  <Chip 
                    label="需要管理员权限" 
                    size="small" 
                    color="warning" 
                    variant="outlined"
                  />
                )}
              </Box>
            }
          />
          <Typography variant="caption" color="text.secondary" sx={{ ml: 4, display: 'block' }}>
            启用后所有网络流量将通过代理，需要管理员权限
          </Typography>
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
