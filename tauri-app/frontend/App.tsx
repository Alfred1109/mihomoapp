import React, { useState, useEffect, useCallback, Suspense, lazy } from 'react';
import {
  Box,
  Tabs,
  Tab,
  Alert,
  Snackbar,
  Button,
  IconButton,
  Menu,
  MenuItem,
  Tooltip,
  CircularProgress,
  Typography,
} from '@mui/material';
import { Language, FiberManualRecord } from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';
import TitleBar from './components/TitleBar';
import { TabPanel, ErrorBoundary } from './components/common';
import { useAppStore } from './store/appStore';
import { isTauriEnv } from './utils/tauri';

const Dashboard = lazy(() => import('./components/Dashboard'));
const SubscriptionManager = lazy(() => import('./components/SubscriptionManager'));
const ProxyManager = lazy(() => import('./components/ProxyManager'));
const ConfigManager = lazy(() => import('./components/ConfigManager'));

const LoadingFallback: React.FC = () => (
  <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', py: 8, gap: 2 }}>
    <CircularProgress size={32} />
    <Typography variant="body2" color="text.secondary">Loading...</Typography>
  </Box>
);

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
    if (!isTauriEnv()) return;
    
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
        }
      }
    } catch {
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
    } catch {
      setAdminCheckDone(true);
    }
  }, [setIsAdmin, setAdminCheckDone]);

  const handleRestartAsAdmin = async () => {
    if (!isTauriEnv()) return;
    
    try {
      await invoke('restart_as_admin');
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
      <Box sx={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
        <TitleBar>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mr: 1 }}>
            <Tooltip title={mihomoStatus.running ? t('dashboard.running') : t('dashboard.stopped')}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, px: 1 }}>
                <FiberManualRecord 
                  sx={{ 
                    fontSize: 10, 
                    color: mihomoStatus.running ? 'success.main' : 'error.main',
                  }} 
                />
              </Box>
            </Tooltip>

            {adminCheckDone && !isAdmin && (
              <Button
                size="small"
                color="warning"
                onClick={handleRestartAsAdmin}
                sx={{ 
                  fontSize: '0.7rem', 
                  py: 0.25, 
                  px: 1,
                  minWidth: 'auto',
                }}
              >
                {t('permissions.restartAsAdmin')}
              </Button>
            )}

            <IconButton
              size="small"
              onClick={handleLanguageMenuOpen}
              sx={{ color: 'text.secondary' }}
            >
              <Language sx={{ fontSize: 18 }} />
            </IconButton>
            <Menu
              anchorEl={langMenuAnchor}
              open={Boolean(langMenuAnchor)}
              onClose={handleLanguageMenuClose}
              anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
              transformOrigin={{ vertical: 'top', horizontal: 'right' }}
            >
              <MenuItem onClick={() => handleLanguageChange('en')} selected={i18n.language === 'en'}>
                English
              </MenuItem>
              <MenuItem onClick={() => handleLanguageChange('zh')} selected={i18n.language === 'zh'}>
                中文
              </MenuItem>
            </Menu>
          </Box>
        </TitleBar>

        <Box sx={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>
          <Box sx={{ borderBottom: 1, borderColor: 'divider', backgroundColor: 'background.paper' }}>
            <Tabs 
              value={tabValue} 
              onChange={handleTabChange} 
              aria-label="mihomo manager tabs"
              sx={{ px: 2, minHeight: 42 }}
            >
              <Tab label={t('app.dashboard')} />
              <Tab label={t('app.subscription')} />
              <Tab label={t('app.proxy')} />
              <Tab label={t('app.config')} />
            </Tabs>
          </Box>

          <Box sx={{ flex: 1, overflow: 'auto', p: 2 }}>
            <TabPanel value={tabValue} index={0}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <Dashboard 
                    isRunning={mihomoStatus.running}
                    onStatusChange={checkMihomoStatus}
                    showNotification={showNotification}
                  />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>

            <TabPanel value={tabValue} index={1}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <SubscriptionManager showNotification={showNotification} />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>

            <TabPanel value={tabValue} index={2}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <ProxyManager 
                    isRunning={mihomoStatus.running}
                    showNotification={showNotification}
                  />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>

            <TabPanel value={tabValue} index={3}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <ConfigManager 
                    isRunning={mihomoStatus.running}
                    showNotification={showNotification}
                  />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>
          </Box>
        </Box>

        <Snackbar
          open={notification.open}
          autoHideDuration={4000}
          onClose={handleCloseNotification}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        >
          <Alert 
            onClose={handleCloseNotification} 
            severity={notification.severity}
            variant="filled"
            sx={{ fontSize: '0.8125rem' }}
          >
            {notification.message}
          </Alert>
        </Snackbar>
      </Box>
    </ErrorBoundary>
  );
}

export default App;
