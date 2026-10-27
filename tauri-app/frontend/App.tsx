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
  useMediaQuery,
} from '@mui/material';
import { Language, FiberManualRecord, SpaceDashboardOutlined, SubscriptionsOutlined, HubOutlined, TuneOutlined } from '@mui/icons-material';
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
  const [visitedTabs, setVisitedTabs] = useState<number[]>([0]);
  const wideLayout = useMediaQuery((theme: import('@mui/material/styles').Theme) => theme.breakpoints.up('md'));
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
    adminRestartPending,
    setIsAdmin, 
    setAdminCheckDone, 
    setAdminRestartPending,
    setMihomoStatus, 
    initEventListeners,
    cleanupEventListeners 
  } = useAppStore();

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setVisitedTabs(previous => previous.includes(newValue) ? previous : [...previous, newValue]);
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
      
      if (isRunning) {
        setMihomoStatus({ running: true, processId: null, timestamp: Date.now() });
      } else {
        try {
          const directStatus = await invoke<boolean>('get_mihomo_status');
          setMihomoStatus({
            running: directStatus,
            processId: null,
            timestamp: Date.now(),
          });
        } catch (error) {
          setMihomoStatus({ running: false, processId: null, timestamp: 0 });
          console.warn('Failed to check mihomo direct status:', error);
        }
      }
    } catch (error) {
      setMihomoStatus({ running: false, processId: null, timestamp: 0 });
      console.warn('Failed to check mihomo service status:', error);
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
      setAdminRestartPending(true);
      await invoke('restart_as_admin');
      window.setTimeout(() => {
        setAdminRestartPending(false);
        showNotification(t('notifications.adminRestartTimeout'), 'warning');
      }, 20000);
    } catch (error) {
      setAdminRestartPending(false);
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
    if (isTauriEnv()) void initEventListeners();
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
            <Tooltip title={!mihomoStatus.timestamp ? t('app.unknownStatus') : mihomoStatus.running ? t('dashboard.running') : t('dashboard.stopped')}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, px: 1 }}>
                <FiberManualRecord 
                  sx={{ 
                    fontSize: 10, 
                    color: !mihomoStatus.timestamp ? 'text.disabled' : mihomoStatus.running ? 'success.main' : 'error.main',
                  }} 
                />
                <Typography variant="caption" sx={{ color: 'text.secondary', display: { xs: 'none', sm: 'inline' }, whiteSpace: 'nowrap' }}>
                  {!mihomoStatus.timestamp ? t('app.unknownStatus') : mihomoStatus.running ? t('dashboard.running') : t('dashboard.stopped')}
                </Typography>
              </Box>
            </Tooltip>

            {isTauriEnv() && adminCheckDone && !isAdmin && (
              <Button
                size="small"
                color="warning"
                onClick={handleRestartAsAdmin}
                disabled={adminRestartPending}
                sx={{ 
                  fontSize: '0.7rem', 
                  py: 0.25, 
                  px: 1,
                  minWidth: 'auto',
                }}
              >
                {adminRestartPending ? t('permissions.restartingAsAdmin') : t('permissions.restartAsAdmin')}
              </Button>
            )}

            <IconButton
              size="small"
              aria-label={t('app.language')}
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
          <Box sx={{ display: 'flex', flex: 1, minHeight: 0, flexDirection: { xs: 'column', md: 'row' } }}>
          <Box component="nav" aria-label={t('app.navigation')} sx={{ width: { md: 210 }, flexShrink: 0, borderRight: { md: 1 }, borderBottom: { xs: 1, md: 0 }, borderColor: 'divider', backgroundColor: 'background.paper', px: { xs: 1, md: 1.5 }, pt: { md: 2.5 } }}>
            <Typography variant="overline" sx={{ display: { xs: 'none', md: 'block' }, px: 1.5, mb: 1, color: 'text.secondary', letterSpacing: '0.12em', fontSize: '0.67rem', fontWeight: 700 }}>{t('app.navigation')}</Typography>
            <Tabs
              value={tabValue}
              onChange={handleTabChange}
              orientation={wideLayout ? 'vertical' : 'horizontal'}
              variant="scrollable"
              allowScrollButtonsMobile
              aria-label={t('app.navigation')}
              sx={{ '& .MuiTabs-indicator': { display: 'none' }, '& .MuiTabs-flexContainer': { gap: 0.5 }, '& .MuiTab-root': { alignItems: 'center', justifyContent: { md: 'flex-start' }, minHeight: 44, px: 1.5, borderRadius: 2, color: 'text.secondary', '&.Mui-selected': { color: 'primary.light', backgroundColor: 'rgba(96, 165, 250, 0.12)' } } }}
            >
              <Tab id="tab-0" aria-controls="tabpanel-0" icon={<SpaceDashboardOutlined />} iconPosition="start" label={t('app.dashboard')} />
              <Tab id="tab-1" aria-controls="tabpanel-1" icon={<SubscriptionsOutlined />} iconPosition="start" label={t('app.subscription')} />
              <Tab id="tab-2" aria-controls="tabpanel-2" icon={<HubOutlined />} iconPosition="start" label={t('app.proxy')} />
              <Tab id="tab-3" aria-controls="tabpanel-3" icon={<TuneOutlined />} iconPosition="start" label={t('app.config')} />
            </Tabs>
          </Box>

          <Box component="main" sx={{ flex: 1, minWidth: 0, overflow: 'auto', px: { xs: 2, md: 3.5 }, py: { xs: 2, md: 3 } }}>
            <Box sx={{ maxWidth: 1440, mx: 'auto' }}>
            <TabPanel value={tabValue} index={0} mounted={visitedTabs.includes(0)}>
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

            <TabPanel value={tabValue} index={1} mounted={visitedTabs.includes(1)}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <SubscriptionManager showNotification={showNotification} />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>

            <TabPanel value={tabValue} index={2} mounted={visitedTabs.includes(2)}>
              <ErrorBoundary>
                <Suspense fallback={<LoadingFallback />}>
                  <ProxyManager 
                    isRunning={mihomoStatus.running}
                    active={tabValue === 2}
                    showNotification={showNotification}
                  />
                </Suspense>
              </ErrorBoundary>
            </TabPanel>

            <TabPanel value={tabValue} index={3} mounted={visitedTabs.includes(3)}>
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
