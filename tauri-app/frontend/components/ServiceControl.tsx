import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Button,
  Box,
  Chip,
  CircularProgress,
  FormControlLabel,
  Switch,
  Divider,
} from '@mui/material';
import {
  PlayArrow,
  Stop,
  Refresh,
  Security,
  AutorenewOutlined,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { isTauriEnv } from '../utils/tauri';

interface ServiceControlProps {
  isRunning: boolean;
  onStatusChange: () => void;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const ServiceControl: React.FC<ServiceControlProps> = React.memo(({ isRunning, onStatusChange, showNotification }) => {
  const [loading, setLoading] = useState(false);
  const [serviceStatus, setServiceStatus] = useState<string>('not_installed');
  const [serviceLoading, setServiceLoading] = useState(false);
  const [autoRestart, setAutoRestart] = useState(true);

  const checkServiceStatus = useCallback(async () => {
    if (!isTauriEnv()) return;
    
    try {
      const status = await invoke<string>('get_mihomo_service_status');
      setServiceStatus(status);
    } catch (error) {
      console.error('Failed to check service status:', error);
    }
  }, []);

  const loadAutoRestartSetting = useCallback(async () => {
    if (!isTauriEnv()) return;
    
    try {
      const enabled = await invoke<boolean>('get_auto_restart');
      setAutoRestart(enabled);
    } catch (error) {
      console.error('Failed to load auto-restart setting:', error);
    }
  }, []);

  useEffect(() => {
    checkServiceStatus();
    loadAutoRestartSetting();
  }, [checkServiceStatus, loadAutoRestartSetting]);

  const handleAutoRestartChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const enabled = event.target.checked;
    setAutoRestart(enabled);
    
    try {
      await invoke('set_auto_restart', { enabled });
      showNotification(
        enabled ? '已启用进程崩溃自动重启' : '已禁用进程崩溃自动重启',
        'success'
      );
    } catch (error) {
      showNotification(`设置失败: ${error}`, 'error');
      setAutoRestart(!enabled);
    }
  };

  const handleStartService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('start_mihomo_service_cmd');
      showNotification(result, 'success');
      await new Promise(resolve => setTimeout(resolve, 1500));
      await checkServiceStatus();
      onStatusChange();
    } catch (error) {
      showNotification(`启动失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const handleStopService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('stop_mihomo_service_cmd');
      showNotification(result, 'success');
      await new Promise(resolve => setTimeout(resolve, 500));
      await checkServiceStatus();
      onStatusChange();
    } catch (error) {
      showNotification(`停止失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const handleRestart = async () => {
    setLoading(true);
    try {
      const result = await invoke<string>('restart_mihomo_service_cmd');
      await new Promise(resolve => setTimeout(resolve, 2000));
      await checkServiceStatus();
      showNotification(result || 'Mihomo重启成功，配置已应用', 'success');
      onStatusChange();
    } catch (error) {
      showNotification(`重启失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleInstallService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('install_mihomo_service');
      showNotification(result, 'success');
      await checkServiceStatus();
    } catch (error) {
      showNotification(`安装失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const handleUninstallService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('uninstall_mihomo_service');
      showNotification(result, 'success');
      await checkServiceStatus();
    } catch (error) {
      showNotification(`卸载失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  return (
    <Card>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          Mihomo 服务管理
        </Typography>
        
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
          <Chip
            label={`服务状态: ${
              serviceStatus === 'running' ? '运行中' :
              serviceStatus === 'stopped' ? '已停止' :
              serviceStatus === 'installed' ? '已安装' :
              serviceStatus === 'not_installed' ? '未安装' : '未知'
            }`}
            color={
              serviceStatus === 'running' ? 'success' :
              serviceStatus === 'installed' ? 'info' : 'default'
            }
            variant="filled"
          />
          {(loading || serviceLoading) && <CircularProgress size={20} />}
        </Box>

        <Box sx={{ mb: 2 }}>
          <FormControlLabel
            control={
              <Switch
                checked={autoRestart}
                onChange={handleAutoRestartChange}
                color="primary"
              />
            }
            label={
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <AutorenewOutlined fontSize="small" />
                <Typography variant="body2">
                  进程崩溃时自动重启 (Watchdog)
                </Typography>
              </Box>
            }
          />
          <Typography variant="caption" color="text.secondary" sx={{ ml: 4, display: 'block' }}>
            启用后，进程意外退出时将在 5 秒内自动重启（最多尝试 5 次/分钟）
          </Typography>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
          {serviceStatus === 'not_installed' && (
            <Button
              variant="contained"
              color="primary"
              onClick={handleInstallService}
              disabled={serviceLoading}
              startIcon={<Security />}
            >
              安装 Mihomo 服务
            </Button>
          )}

          {(serviceStatus === 'installed' || serviceStatus === 'stopped') && (
            <>
              <Button
                variant="contained"
                color="success"
                onClick={handleStartService}
                disabled={serviceLoading}
                startIcon={<PlayArrow />}
              >
                启动服务
              </Button>
              <Button
                variant="outlined"
                color="error"
                onClick={handleUninstallService}
                disabled={serviceLoading}
                startIcon={<Stop />}
              >
                卸载服务
              </Button>
            </>
          )}

          {serviceStatus === 'running' && (
            <>
              <Button
                variant="contained"
                color="warning"
                onClick={handleRestart}
                disabled={serviceLoading || loading}
                startIcon={<Refresh />}
              >
                重启服务
              </Button>
              <Button
                variant="contained"
                color="error"
                onClick={handleStopService}
                disabled={serviceLoading}
                startIcon={<Stop />}
              >
                停止服务
              </Button>
            </>
          )}
        </Box>
      </CardContent>
    </Card>
  );
});

ServiceControl.displayName = 'ServiceControl';

export default ServiceControl;
