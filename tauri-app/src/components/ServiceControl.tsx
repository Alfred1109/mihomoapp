import React, { useState } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Button,
  Box,
  Chip,
  CircularProgress,
} from '@mui/material';
import {
  PlayArrow,
  Stop,
  Refresh,
  Security,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface ServiceControlProps {
  isRunning: boolean;
  onStatusChange: (status: boolean) => void;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const ServiceControl: React.FC<ServiceControlProps> = ({ isRunning, onStatusChange, showNotification }) => {
  const [loading, setLoading] = useState(false);
  const [serviceStatus, setServiceStatus] = useState<string>('not_installed');
  const [serviceLoading, setServiceLoading] = useState(false);

  React.useEffect(() => {
    checkServiceStatus();
  }, []);

  const checkServiceStatus = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    try {
      const status = await invoke<string>('get_mihomo_service_status');
      setServiceStatus(status);
    } catch (error) {
      console.error('Failed to check service status:', error);
    }
  };

  const handleStartService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('start_mihomo_service_cmd');
      showNotification(result, 'success');
      // 等待服务完全启动
      await new Promise(resolve => setTimeout(resolve, 1500));
      await checkServiceStatus();
      // 立即更新父组件状态
      onStatusChange(true);
    } catch (error) {
      showNotification(`启动失败: ${error}`, 'error');
      onStatusChange(false);
    } finally {
      setServiceLoading(false);
    }
  };

  const handleStopService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('stop_mihomo_service_cmd');
      showNotification(result, 'success');
      // 等待服务完全停止
      await new Promise(resolve => setTimeout(resolve, 500));
      await checkServiceStatus();
      // 立即更新父组件状态
      onStatusChange(false);
    } catch (error) {
      showNotification(`停止失败: ${error}`, 'error');
      onStatusChange(false);
    } finally {
      setServiceLoading(false);
    }
  };

  const handleRestart = async () => {
    setLoading(true);
    try {
      const result = await invoke<string>('restart_mihomo_service_cmd');
      // 等待服务完全重启
      await new Promise(resolve => setTimeout(resolve, 2000));
      await checkServiceStatus();
      showNotification(result || 'Mihomo重启成功，配置已应用', 'success');
      // 立即更新父组件状态
      onStatusChange(true);
    } catch (error) {
      showNotification(`重启失败: ${error}`, 'error');
      onStatusChange(false);
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
        
        {/* Service Status */}
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

        {/* Service Management Buttons */}
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
          {/* 未安装时：显示安装按钮 */}
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

          {/* 已安装或已停止时：显示启动和卸载按钮 */}
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

          {/* 运行中时：显示重启和停止按钮 */}
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
};

export default ServiceControl;
