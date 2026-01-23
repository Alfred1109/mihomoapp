import React, { useState, useEffect } from 'react';
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
import BackupManager from './components/BackupManager';

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
}

function TabPanel(props: TabPanelProps) {
  const { children, value, index, ...other } = props;

  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`simple-tabpanel-${index}`}
      aria-labelledby={`simple-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

function App() {
  const { t, i18n } = useTranslation();
  const [tabValue, setTabValue] = useState(0);
  const [mihomoStatus, setMihomoStatus] = useState(false);
  const [isAdmin, setIsAdmin] = useState(false);
  const [adminCheckDone, setAdminCheckDone] = useState(false);
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

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  const checkMihomoStatus = async () => {
    // Check if we're running in Tauri environment
    if (typeof window !== 'undefined' && (window as any).__TAURI_IPC__) {
      try {
        const serviceStatus = await invoke<string>('get_mihomo_service_status');
        if (serviceStatus === 'running') {
          setMihomoStatus(true);
          return;
        }

        // Fallback to direct process status
        const status = await invoke<boolean>('get_mihomo_status');
        setMihomoStatus(status);
      } catch (error) {
        console.error('Failed to get mihomo status:', error);
      }
    } else {
      console.log('Running in browser mode - Tauri API not available');
      // Set mock status for browser development
      setMihomoStatus(false);
    }
  };

  const showNotification = (message: string, severity: 'success' | 'error' | 'info' | 'warning' = 'info') => {
    setNotification({ open: true, message, severity });
  };

  const handleCloseNotification = () => {
    setNotification({ ...notification, open: false });
  };

  const checkAdminPrivileges = async () => {
    if (typeof window !== 'undefined' && (window as any).__TAURI_IPC__) {
      try {
        const adminStatus = await invoke<boolean>('check_admin_privileges');
        setIsAdmin(adminStatus);
        setAdminCheckDone(true);
        
        if (!adminStatus) {
          showNotification('应用未以 root 权限运行，某些功能可能受限', 'warning');
        }
      } catch (error) {
        console.error('Failed to check admin privileges:', error);
        setAdminCheckDone(true);
      }
    } else {
      setAdminCheckDone(true);
    }
  };

  const handleRestartAsAdmin = async () => {
    if (typeof window !== 'undefined' && (window as any).__TAURI_IPC__) {
      try {
        await invoke('restart_as_admin');
        showNotification('重启失败，请手动使用 sudo 运行应用', 'error');
      } catch (error) {
        showNotification(`重启失败: ${error}`, 'error');
      }
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
    checkMihomoStatus();
    checkAdminPrivileges();
    
    // Check status periodically
    const interval = setInterval(checkMihomoStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
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
                label={isAdmin ? 'Root 权限' : '普通权限'}
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
                以 Root 权限重启
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
                backgroundColor: mihomoStatus ? 'success.main' : 'error.main',
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
                {mihomoStatus ? t('dashboard.running') : t('dashboard.stopped')}
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
            <Tab label="备份管理" />
          </Tabs>
        </Box>

        <TabPanel value={tabValue} index={0}>
          <Dashboard 
            isRunning={mihomoStatus}
            onStatusChange={setMihomoStatus}
            showNotification={showNotification}
          />
        </TabPanel>

        <TabPanel value={tabValue} index={1}>
          <SubscriptionManager showNotification={showNotification} />
        </TabPanel>

        <TabPanel value={tabValue} index={2}>
          <ProxyManager 
            isRunning={mihomoStatus}
            showNotification={showNotification}
          />
        </TabPanel>

        <TabPanel value={tabValue} index={3}>
          <ConfigManager 
            isRunning={mihomoStatus}
            showNotification={showNotification}
          />
        </TabPanel>

        <TabPanel value={tabValue} index={4}>
          <BackupManager showNotification={showNotification} />
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
  );
}

export default App;
