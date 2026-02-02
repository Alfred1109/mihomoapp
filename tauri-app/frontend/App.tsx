import React, { useState, useEffect, useCallback } from 'react';
import {
  AppBar,
  Toolbar,
  Typography,
  Container,
  Box,
  Tabs,
  Tab,
  Alert,
  Snackbar,
  Button,
  Chip,
  IconButton,
  Menu,
  MenuItem,
} from '@mui/material';
import { Language } from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';
import Dashboard from './components/Dashboard';
import SubscriptionManager from './components/SubscriptionManager';
import ProxyManager from './components/ProxyManager';
import ConfigManager from './components/ConfigManager';
import { TabPanel, ErrorBoundary } from './components/common';
import { useAppStore } from './store/appStore';
import { isTauriEnv } from './utils/tauri';

function App() {
  const { t, i18n } = useTranslation();
  const [tabValue, setTabValue] = useState(0);
  const [langMenuAnchor, setLangMenuAnchor] = useState<null | HTMLElement>(null);
  const [notification, setNotification] = useState<{
    open: boolean;
    message: string;
    severity: 'success' | 'error' | 'info' | 'warning';
  }>({
    open: false,
    message: '',
    severity: 'info',
  });

  const { 
    mihomoStatus, 
    isAdmin, 
    adminCheckDone, 
    setIsAdmin, 
    setAdminCheckDone, 
    setMihomoStatus, 
    initEventListeners,
    cleanupEventListeners 
  } = useAppStore();

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  const showNotification = useCallback((message: string, severity: 'success' | 'error' | 'info' | 'warning' = 'info') => {
    setNotification({ open: true, message, severity });
  }, []);

  const handleCloseNotification = useCallback(() => {
    setNotification(prev => ({ ...prev, open: false }));
  }, []);

  const checkMihomoStatus = useCallback(async () => {
    if (!isTauriEnv()) {
      return;
    }
    
    try {
      const serviceStatus = await invoke<string>('get_mihomo_service_status');
      const isRunning = serviceStatus === 'running';
      
      setMihomoStatus({
        running: isRunning,
        processId: null,
        timestamp: Date.now(),
      });
      
      if (!isRunning) {
        try {
          const directStatus = await invoke<boolean>('get_mihomo_status');
          setMihomoStatus({
            running: directStatus,
            processId: null,
            timestamp: Date.now(),
          });
        } catch {
          // Ignore error
        }
      }
    } catch (error) {
      console.error('Failed to get mihomo status:', error);
    }
  }, [setMihomoStatus]);

  const checkAdminPrivileges = useCallback(async () => {
    if (!isTauriEnv()) {
      setAdminCheckDone(true);
      return;
    }
    
    try {
      const adminStatus = await invoke<boolean>('check_admin_privileges');
      setIsAdmin(adminStatus);
      setAdminCheckDone(true);
      
      if (!adminStatus) {
        showNotification(t('permissions.requiresAdmin'), 'warning');
      }
    } catch (error) {
      console.error('Failed to check admin privileges:', error);
      setAdminCheckDone(true);
    }
  }, [setIsAdmin, setAdminCheckDone, showNotification, t]);

  const handleRestartAsAdmin = async () => {
    if (!isTauriEnv()) return;
    
    try {
      await invoke('restart_as_admin');
      showNotification(t('notifications.startError'), 'error');
    } catch (error) {
      showNotification(`${t('notifications.startError')}: ${error}`, 'error');
    }
  };

  const handleLanguageMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setLangMenuAnchor(event.currentTarget);
  };

  const handleLanguageMenuClose = () => {
    setLangMenuAnchor(null);
  };

  const handleLanguageChange = (lang: string) => {
    i18n.changeLanguage(lang);
    localStorage.setItem('language', lang);
    handleLanguageMenuClose();
  };

  useEffect(() => {
    initEventListeners();
    checkMihomoStatus();
    checkAdminPrivileges();

    return () => {
      cleanupEventListeners();
    };
  }, [initEventListeners, cleanupEventListeners, checkMihomoStatus, checkAdminPrivileges]);

  return (
    <ErrorBoundary>
      <Box sx={{ flexGrow: 1 }}>
        <AppBar position="static" elevation={0}>
          <Toolbar>
            <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
              {t('app.title')}
            </Typography>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <IconButton
                color="inherit"
                onClick={handleLanguageMenuOpen}
                size="small"
                title="Language"
              >
                <Language />
              </IconButton>
              <Menu
                anchorEl={langMenuAnchor}
                open={Boolean(langMenuAnchor)}
                onClose={handleLanguageMenuClose}
              >
                <MenuItem onClick={() => handleLanguageChange('en')} selected={i18n.language === 'en'}>
                  English
                </MenuItem>
                <MenuItem onClick={() => handleLanguageChange('zh')} selected={i18n.language === 'zh'}>
                  中文
                </MenuItem>
              </Menu>
              {adminCheckDone && (
                <Chip
                  label={isAdmin ? t('permissions.admin') : t('permissions.normal')}
                  color={isAdmin ? 'success' : 'warning'}
                  size="small"
                  variant="filled"
                />
              )}
              {adminCheckDone && !isAdmin && (
                <Button
                  variant="outlined"
                  size="small"
                  color="warning"
                  onClick={handleRestartAsAdmin}
                  sx={{ color: 'white', borderColor: 'white' }}
                >
                  {t('permissions.restartAsAdmin')}
                </Button>
              )}
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 1,
                  px: 2,
                  py: 0.5,
                  borderRadius: 1,
                  backgroundColor: mihomoStatus.running ? 'success.main' : 'error.main',
                  color: 'white',
                }}
              >
                <Box
                  sx={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    backgroundColor: 'currentColor',
                  }}
                />
                <Typography variant="body2">
                  {mihomoStatus.running ? t('dashboard.running') : t('dashboard.stopped')}
                </Typography>
              </Box>
            </Box>
          </Toolbar>
        </AppBar>

        <Container maxWidth="xl" sx={{ mt: 2 }}>
          <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
            <Tabs value={tabValue} onChange={handleTabChange} aria-label="mihomo manager tabs">
              <Tab label={t('app.dashboard')} />
              <Tab label={t('app.subscription')} />
              <Tab label={t('app.proxy')} />
              <Tab label={t('app.config')} />
            </Tabs>
          </Box>

          <TabPanel value={tabValue} index={0}>
            <ErrorBoundary>
              <Dashboard 
                isRunning={mihomoStatus.running}
                onStatusChange={checkMihomoStatus}
                showNotification={showNotification}
              />
            </ErrorBoundary>
          </TabPanel>

          <TabPanel value={tabValue} index={1}>
            <ErrorBoundary>
              <SubscriptionManager showNotification={showNotification} />
            </ErrorBoundary>
          </TabPanel>

          <TabPanel value={tabValue} index={2}>
            <ErrorBoundary>
              <ProxyManager 
                isRunning={mihomoStatus.running}
                showNotification={showNotification}
              />
            </ErrorBoundary>
          </TabPanel>

          <TabPanel value={tabValue} index={3}>
            <ErrorBoundary>
              <ConfigManager 
                isRunning={mihomoStatus.running}
                showNotification={showNotification}
              />
            </ErrorBoundary>
          </TabPanel>
        </Container>

        <Snackbar
          open={notification.open}
          autoHideDuration={6000}
          onClose={handleCloseNotification}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        >
          <Alert 
            onClose={handleCloseNotification} 
            severity={notification.severity}
            variant="filled"
          >
            {notification.message}
          </Alert>
        </Snackbar>
      </Box>
    </ErrorBoundary>
  );
}

export default App;
