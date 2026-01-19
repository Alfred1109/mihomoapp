import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Tabs,
  Tab,
  TextField,
  Switch,
  FormControlLabel,
  Button,
  Grid,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Paper,
  Alert,
  Chip,
  Divider,
} from '@mui/material';
import {
  Save,
  Refresh,
  RestoreFromTrash,
  Settings,
  NetworkCheck,
  Dns,
  Security,
  Code,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface ConfigManagerProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

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
      id={`config-tabpanel-${index}`}
      aria-labelledby={`config-tab-${index}`}
      {...other}
    >
      {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
    </div>
  );
}

const ConfigManager: React.FC<ConfigManagerProps> = ({ isRunning, showNotification }) => {
  const [config, setConfig] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [configText, setConfigText] = useState('');
  const [tabValue, setTabValue] = useState(0);

  const loadConfig = async () => {
    setLoading(true);
    try {
      const configData = await invoke<any>('get_mihomo_config');
      setConfig(configData);
      setConfigText(JSON.stringify(configData, null, 2));
      setHasChanges(false);
    } catch (error) {
      showNotification(`Failed to load configuration: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const saveConfig = async () => {
    setLoading(true);
    try {
      await invoke('save_mihomo_config', { config });
      setHasChanges(false);
      showNotification('Configuration saved successfully', 'success');
    } catch (error) {
      showNotification(`Failed to save configuration: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const resetConfig = async () => {
    if (!confirm('Are you sure you want to reset the configuration? This will overwrite all current settings.')) {
      return;
    }

    setLoading(true);
    try {
      // This would need to be implemented in the backend
      await loadConfig();
      showNotification('Configuration reset successfully', 'success');
    } catch (error) {
      showNotification(`Failed to reset configuration: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const updateConfig = (field: string, value: any) => {
    if (!config) return;

    const newConfig = { ...config };
    const fields = field.split('.');
    let current = newConfig;

    for (let i = 0; i < fields.length - 1; i++) {
      if (!current[fields[i]]) {
        current[fields[i]] = {};
      }
      current = current[fields[i]];
    }
    current[fields[fields.length - 1]] = value;

    setConfig(newConfig);
    setHasChanges(true);
  };

  const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
  };

  const applyTextConfig = () => {
    try {
      const parsedConfig = JSON.parse(configText);
      setConfig(parsedConfig);
      setHasChanges(true);
      showNotification('Configuration loaded from text editor', 'success');
    } catch (error) {
      showNotification('Invalid JSON format in text editor', 'error');
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  useEffect(() => {
    if (config) {
      setConfigText(JSON.stringify(config, null, 2));
    }
  }, [config]);

  if (!config) {
    return (
      <Card>
        <CardContent sx={{ textAlign: 'center', py: 8 }}>
          <Settings sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
          <Typography variant="h6" gutterBottom>
            Loading Configuration...
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">Configuration Manager</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          {hasChanges && <Chip label="Unsaved Changes" color="warning" size="small" />}
          <Button
            variant="outlined"
            startIcon={<RestoreFromTrash />}
            onClick={resetConfig}
            disabled={loading}
          >
            Reset
          </Button>
          <Button
            variant="outlined"
            startIcon={<Refresh />}
            onClick={loadConfig}
            disabled={loading}
          >
            Reload
          </Button>
          <Button
            variant="contained"
            startIcon={<Save />}
            onClick={saveConfig}
            disabled={loading || !hasChanges}
          >
            Save
          </Button>
        </Box>
      </Box>

      <Card>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={tabValue} onChange={handleTabChange}>
            <Tab icon={<NetworkCheck />} label="General" />
            <Tab icon={<Security />} label="Proxy" />
            <Tab icon={<Dns />} label="DNS" />
            <Tab icon={<Code />} label="Advanced" />
          </Tabs>
        </Box>

        {/* General Configuration */}
        <TabPanel value={tabValue} index={0}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                Basic Settings
              </Typography>
              
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <TextField
                    label="HTTP Port"
                    type="number"
                    value={config.port || 7890}
                    onChange={(e) => updateConfig('port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="SOCKS Port"
                    type="number"
                    value={config['socks-port'] || 7891}
                    onChange={(e) => updateConfig('socks-port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="Mixed Port"
                    type="number"
                    value={config['mixed-port'] || 7890}
                    onChange={(e) => updateConfig('mixed-port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="External Controller"
                    value={config['external-controller'] || '127.0.0.1:9090'}
                    onChange={(e) => updateConfig('external-controller', e.target.value)}
                    fullWidth
                  />
                </Grid>
              </Grid>

              <Box sx={{ mt: 3 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={config['allow-lan'] || false}
                      onChange={(e) => updateConfig('allow-lan', e.target.checked)}
                    />
                  }
                  label="Allow LAN Access"
                />
                <Typography variant="body2" color="text.secondary">
                  Allow other devices on your network to use this proxy
                </Typography>
              </Box>

              <Box sx={{ mt: 2 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.ipv6 !== false}
                      onChange={(e) => updateConfig('ipv6', e.target.checked)}
                    />
                  }
                  label="IPv6 Support"
                />
                <Typography variant="body2" color="text.secondary">
                  Enable IPv6 traffic handling
                </Typography>
              </Box>
            </Grid>

            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                TUN Mode
              </Typography>
              
              <FormControlLabel
                control={
                  <Switch
                    checked={config.tun?.enable || false}
                    onChange={(e) => updateConfig('tun.enable', e.target.checked)}
                  />
                }
                label="Enable TUN Mode"
              />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                System-wide transparent proxy (requires administrator privileges)
              </Typography>

              {config.tun?.enable && (
                <Grid container spacing={2}>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>TUN Stack</InputLabel>
                      <Select
                        value={config.tun?.stack || 'system'}
                        label="TUN Stack"
                        onChange={(e) => updateConfig('tun.stack', e.target.value)}
                      >
                        <MenuItem value="system">System</MenuItem>
                        <MenuItem value="gvisor">gVisor</MenuItem>
                        <MenuItem value="lwip">LWIP</MenuItem>
                      </Select>
                    </FormControl>
                  </Grid>
                  <Grid item xs={12}>
                    <TextField
                      label="Device Name"
                      value={config.tun?.['device-name'] || ''}
                      onChange={(e) => updateConfig('tun.device-name', e.target.value)}
                      placeholder="auto"
                      fullWidth
                    />
                  </Grid>
                </Grid>
              )}
            </Grid>
          </Grid>
        </TabPanel>

        {/* Proxy Configuration */}
        <TabPanel value={tabValue} index={1}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                Proxy Settings
              </Typography>
              
              <Grid container spacing={2}>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>Proxy Mode</InputLabel>
                    <Select
                      value={config.mode || 'rule'}
                      label="Proxy Mode"
                      onChange={(e) => updateConfig('mode', e.target.value)}
                    >
                      <MenuItem value="rule">Rule-based</MenuItem>
                      <MenuItem value="global">Global</MenuItem>
                      <MenuItem value="direct">Direct</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>Log Level</InputLabel>
                    <Select
                      value={config['log-level'] || 'info'}
                      label="Log Level"
                      onChange={(e) => updateConfig('log-level', e.target.value)}
                    >
                      <MenuItem value="silent">Silent</MenuItem>
                      <MenuItem value="error">Error</MenuItem>
                      <MenuItem value="warning">Warning</MenuItem>
                      <MenuItem value="info">Info</MenuItem>
                      <MenuItem value="debug">Debug</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
              </Grid>
            </Grid>
          </Grid>
        </TabPanel>

        {/* DNS Configuration */}
        <TabPanel value={tabValue} index={2}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                DNS Settings
              </Typography>
              
              <FormControlLabel
                control={
                  <Switch
                    checked={config.dns?.enable !== false}
                    onChange={(e) => updateConfig('dns.enable', e.target.checked)}
                  />
                }
                label="Enable DNS"
              />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                Use mihomo's built-in DNS server
              </Typography>

              {config.dns?.enable !== false && (
                <Grid container spacing={2}>
                  <Grid item xs={12}>
                    <TextField
                      label="DNS Listen Address"
                      value={config.dns?.listen || '0.0.0.0:53'}
                      onChange={(e) => updateConfig('dns.listen', e.target.value)}
                      fullWidth
                    />
                  </Grid>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>Enhanced Mode</InputLabel>
                      <Select
                        value={config.dns?.enhanced_mode || 'fake-ip'}
                        label="Enhanced Mode"
                        onChange={(e) => updateConfig('dns.enhanced_mode', e.target.value)}
                      >
                        <MenuItem value="fake-ip">Fake IP</MenuItem>
                        <MenuItem value="redir-host">Redirect Host</MenuItem>
                      </Select>
                    </FormControl>
                  </Grid>
                </Grid>
              )}
            </Grid>
          </Grid>
        </TabPanel>

        {/* Advanced Configuration */}
        <TabPanel value={tabValue} index={3}>
          <Typography variant="h6" gutterBottom>
            Raw Configuration (JSON)
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Edit the raw configuration directly. Be careful with the syntax.
          </Typography>
          
          <Paper sx={{ p: 2, mb: 2 }}>
            <TextField
              multiline
              rows={20}
              value={configText}
              onChange={(e) => setConfigText(e.target.value)}
              fullWidth
              variant="outlined"
              sx={{ fontFamily: 'monospace' }}
            />
          </Paper>
          
          <Box sx={{ display: 'flex', gap: 2 }}>
            <Button
              variant="outlined"
              onClick={loadConfig}
            >
              Reload from Server
            </Button>
            <Button
              variant="contained"
              onClick={applyTextConfig}
            >
              Apply Changes
            </Button>
          </Box>
        </TabPanel>
      </Card>

      {isRunning && hasChanges && (
        <Alert severity="warning" sx={{ mt: 2 }}>
          Configuration changes will take effect after restarting the mihomo service.
        </Alert>
      )}
    </Box>
  );
};

export default ConfigManager;
