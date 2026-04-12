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
  Dialog,
  DialogTitle,
  DialogContent,
  DialogContentText,
  DialogActions,
  Alert,
} from '@mui/material';
import {
  PlayArrow,
  Stop,
  Refresh,
  Security,
  AutorenewOutlined,
  DeleteForever,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';
import { isTauriEnv } from '../utils/tauri';

interface ServiceControlProps {
  isRunning: boolean;
  onStatusChange: () => void;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

type ServiceStatus = 'running' | 'stopped' | 'not_installed';

const ServiceControl: React.FC<ServiceControlProps> = React.memo(({ isRunning, onStatusChange, showNotification }) => {
  void isRunning;
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>('not_installed');
  const [serviceLoading, setServiceLoading] = useState(false);
  const [autoRestart, setAutoRestart] = useState(true);
  const [uninstallDialogOpen, setUninstallDialogOpen] = useState(false);

  const checkServiceStatus = useCallback(async () => {
    if (!isTauriEnv()) return;
    
    try {
      const status = await invoke<string>('get_mihomo_service_status');
      if (status === 'running') {
        setServiceStatus('running');
      } else if (status === 'stopped' || status === 'installed') {
        setServiceStatus('stopped');
      } else {
        setServiceStatus('not_installed');
      }
    } catch {
      setServiceStatus('not_installed');
    }
  }, []);

  const loadAutoRestartSetting = useCallback(async () => {
    if (!isTauriEnv()) return;
    
    try {
      const enabled = await invoke<boolean>('get_auto_restart');
      setAutoRestart(enabled);
    } catch (error) {
      console.warn('Failed to load auto restart setting:', error);
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
        enabled ? t('service.autoRestartEnabled') : t('service.autoRestartDisabled'),
        'success'
      );
    } catch (error) {
      showNotification(`${t('service.settingFailed')}: ${error}`, 'error');
      setAutoRestart(!enabled);
    }
  };

  const handleInstallService = async () => {
    setServiceLoading(true);
    try {
      const result = await invoke<string>('install_mihomo_service');
      showNotification(result, 'success');
      await checkServiceStatus();
      onStatusChange();
    } catch (error) {
      showNotification(`${t('service.installFailed')}: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
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
      showNotification(`${t('service.startFailed')}: ${error}`, 'error');
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
      showNotification(`${t('service.stopFailed')}: ${error}`, 'error');
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
      showNotification(result || t('service.restartSuccess'), 'success');
      onStatusChange();
    } catch (error) {
      showNotification(`${t('service.restartFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleUninstallService = async () => {
    setUninstallDialogOpen(false);
    setServiceLoading(true);
    try {
      const result = await invoke<string>('uninstall_mihomo_service');
      showNotification(result, 'success');
      await checkServiceStatus();
      onStatusChange();
    } catch (error) {
      showNotification(`${t('service.uninstallFailed')}: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const getStatusLabel = () => {
    switch (serviceStatus) {
      case 'running': return t('service.statusRunning');
      case 'stopped': return t('service.statusStopped');
      case 'not_installed': return t('service.statusNotInstalled');
      default: return t('service.statusUnknown');
    }
  };

  const getStatusColor = (): 'success' | 'warning' | 'default' => {
    switch (serviceStatus) {
      case 'running': return 'success';
      case 'stopped': return 'warning';
      default: return 'default';
    }
  };

  return (
    <Card>
      <CardContent>
        <Typography variant="h6" gutterBottom>
          {t('service.title')}
        </Typography>
        
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
          <Chip
            label={`${t('service.status')}: ${getStatusLabel()}`}
            color={getStatusColor()}
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
                  {t('service.autoRestart')}
                </Typography>
              </Box>
            }
          />
          <Typography variant="caption" color="text.secondary" sx={{ ml: 4, display: 'block' }}>
            {t('service.autoRestartDesc')}
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
              {t('service.install')}
            </Button>
          )}

          {serviceStatus === 'stopped' && (
            <>
              <Button
                variant="contained"
                color="success"
                onClick={handleStartService}
                disabled={serviceLoading}
                startIcon={<PlayArrow />}
              >
                {t('service.start')}
              </Button>
              <Button
                variant="outlined"
                color="error"
                onClick={() => setUninstallDialogOpen(true)}
                disabled={serviceLoading}
                startIcon={<DeleteForever />}
              >
                {t('service.uninstall')}
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
                {t('service.restart')}
              </Button>
              <Button
                variant="contained"
                color="error"
                onClick={handleStopService}
                disabled={serviceLoading}
                startIcon={<Stop />}
              >
                {t('service.stop')}
              </Button>
            </>
          )}
        </Box>
      </CardContent>

      <Dialog open={uninstallDialogOpen} onClose={() => setUninstallDialogOpen(false)}>
        <DialogTitle>{t('service.uninstallConfirmTitle')}</DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>
            {t('service.uninstallWarning')}
          </Alert>
          <DialogContentText>
            {t('service.uninstallConfirmDesc')}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setUninstallDialogOpen(false)}>{t('common.cancel')}</Button>
          <Button variant="contained" color="error" onClick={handleUninstallService}>
            {t('service.confirmUninstall')}
          </Button>
        </DialogActions>
      </Dialog>
    </Card>
  );
});

ServiceControl.displayName = 'ServiceControl';

export default ServiceControl;
