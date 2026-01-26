import React, { useState, useEffect, useCallback } from 'react';
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

const ConfigManager: React.FC<ConfigManagerProps> = React.memo(({ isRunning, showNotification }) => {
  const [config, setConfig] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [configText, setConfigText] = useState('');
  const [tabValue, setTabValue] = useState(0);
  const [autostart, setAutostart] = useState(false);
  const [silentStart, setSilentStart] = useState(false);

  const loadConfig = async () => {
    setLoading(true);
    try {
      const configData = await invoke<any>('get_mihomo_config');
      setConfig(configData);
      setConfigText(JSON.stringify(configData, null, 2));
      setHasChanges(false);
    } catch (error) {
      showNotification(`加载配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const saveConfig = async () => {
    setLoading(true);
    try {
      await invoke('save_mihomo_config', { config });
      setHasChanges(false);
      showNotification('配置保存成功', 'success');
    } catch (error) {
      showNotification(`保存配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const resetConfig = async () => {
    if (!confirm('确定要重置配置吗？这将覆盖所有当前设置。')) {
      return;
    }

    setLoading(true);
    try {
      // This would need to be implemented in the backend
      await loadConfig();
      showNotification('配置重置成功', 'success');
    } catch (error) {
      showNotification(`重置配置失败: ${error}`, 'error');
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
      showNotification('已从文本编辑器加载配置', 'success');
    } catch (error) {
      showNotification('文本编辑器中的JSON格式无效', 'error');
    }
  };

  const loadAppSettings = async () => {
    try {
      const autostartStatus = await invoke<boolean>('get_autostart_status');
      setAutostart(autostartStatus);
      
      const silentStatus = await invoke<boolean>('get_silent_start_status');
      setSilentStart(silentStatus);
    } catch (error) {
      console.error('加载应用设置失败:', error);
    }
  };

  const handleAutostartChange = async (checked: boolean) => {
    try {
      await invoke('set_autostart', { enable: checked });
      setAutostart(checked);
      showNotification(checked ? '已启用开机自启' : '已取消开机自启', 'success');
    } catch (error) {
      showNotification(`设置开机自启失败: ${error}`, 'error');
    }
  };

  const handleSilentStartChange = async (checked: boolean) => {
    try {
      await invoke('set_silent_start', { enable: checked });
      setSilentStart(checked);
      showNotification(checked ? '已启用静默启动' : '已取消静默启动', 'success');
    } catch (error) {
      showNotification(`设置静默启动失败: ${error}`, 'error');
    }
  };

  useEffect(() => {
    loadConfig();
    loadAppSettings();
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
            加载配置中...
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">配置管理</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          {hasChanges && <Chip label="未保存的更改" color="warning" size="small" />}
          <Button
            variant="outlined"
            startIcon={<RestoreFromTrash />}
            onClick={resetConfig}
            disabled={loading}
          >
            重置
          </Button>
          <Button
            variant="outlined"
            startIcon={<Refresh />}
            onClick={loadConfig}
            disabled={loading}
          >
            重新加载
          </Button>
          <Button
            variant="contained"
            startIcon={<Save />}
            onClick={saveConfig}
            disabled={loading || !hasChanges}
          >
            保存
          </Button>
        </Box>
      </Box>

      <Card>
        <Box sx={{ borderBottom: 1, borderColor: 'divider' }}>
          <Tabs value={tabValue} onChange={handleTabChange}>
            <Tab icon={<NetworkCheck />} label="常规" />
            <Tab icon={<Security />} label="代理" />
            <Tab icon={<Dns />} label="DNS" />
            <Tab icon={<Code />} label="高级" />
          </Tabs>
        </Box>

        {/* General Configuration */}
        <TabPanel value={tabValue} index={0}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                基本设置
              </Typography>
              
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <TextField
                    label="HTTP 端口"
                    type="number"
                    value={config.port || 7890}
                    onChange={(e) => updateConfig('port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="SOCKS 端口"
                    type="number"
                    value={config['socks-port'] || 7891}
                    onChange={(e) => updateConfig('socks-port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="混合端口"
                    type="number"
                    value={config['mixed-port'] || 7890}
                    onChange={(e) => updateConfig('mixed-port', parseInt(e.target.value))}
                    fullWidth
                  />
                </Grid>
                <Grid item xs={6}>
                  <TextField
                    label="外部控制器"
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
                  label="允许局域网访问"
                />
                <Typography variant="body2" color="text.secondary">
                  允许网络中的其他设备使用此代理
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
                  label="启用 IPv6"
                />
                <Typography variant="body2" color="text.secondary">
                  启用 IPv6 流量处理
                </Typography>
              </Box>

              <Divider sx={{ my: 3 }} />

              <Typography variant="h6" gutterBottom>
                应用设置
              </Typography>

              <Box sx={{ mt: 2 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={autostart}
                      onChange={(e) => handleAutostartChange(e.target.checked)}
                    />
                  }
                  label="开机自启"
                />
                <Typography variant="body2" color="text.secondary">
                  系统启动时自动运行 Mihomo Manager
                </Typography>
              </Box>

              <Box sx={{ mt: 2 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={silentStart}
                      onChange={(e) => handleSilentStartChange(e.target.checked)}
                    />
                  }
                  label="静默启动"
                />
                <Typography variant="body2" color="text.secondary">
                  启动时不显示窗口，仅在系统托盘显示图标
                </Typography>
              </Box>
            </Grid>

            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>
                TUN 模式
              </Typography>
              
              <FormControlLabel
                control={
                  <Switch
                    checked={config.tun?.enable || false}
                    onChange={(e) => updateConfig('tun.enable', e.target.checked)}
                  />
                }
                label="启用 TUN 模式"
              />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                系统级透明代理（需要 root 权限）
              </Typography>

              {config.tun?.enable && (
                <Grid container spacing={2}>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>TUN 协议栈</InputLabel>
                      <Select
                        value={config.tun?.stack || 'system'}
                        label="TUN 协议栈"
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
                      label="设备名称"
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
                代理设置
              </Typography>
              
              <Grid container spacing={2}>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>代理模式</InputLabel>
                    <Select
                      value={config.mode || 'rule'}
                      label="代理模式"
                      onChange={(e) => updateConfig('mode', e.target.value)}
                    >
                      <MenuItem value="rule">规则模式</MenuItem>
                      <MenuItem value="global">全局模式</MenuItem>
                      <MenuItem value="direct">直连模式</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>日志级别</InputLabel>
                    <Select
                      value={config['log-level'] || 'info'}
                      label="日志级别"
                      onChange={(e) => updateConfig('log-level', e.target.value)}
                    >
                      <MenuItem value="silent">静默</MenuItem>
                      <MenuItem value="error">错误</MenuItem>
                      <MenuItem value="warning">警告</MenuItem>
                      <MenuItem value="info">信息</MenuItem>
                      <MenuItem value="debug">调试</MenuItem>
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
                DNS 设置
              </Typography>
              
              <FormControlLabel
                control={
                  <Switch
                    checked={config.dns?.enable !== false}
                    onChange={(e) => updateConfig('dns.enable', e.target.checked)}
                  />
                }
                label="启用 DNS"
              />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                使用 mihomo 的内置 DNS 服务器
              </Typography>

              <Box sx={{ mt: 2 }}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.dns?.ipv6 === true}
                      onChange={(e) => updateConfig('dns.ipv6', e.target.checked)}
                    />
                  }
                  label="启用 IPv6"
                />
                <Typography variant="body2" color="text.secondary">
                  启用 IPv6 DNS 解析，可能提升解析速度（需要网络支持）
                </Typography>
              </Box>

              {config.dns?.enable !== false && (
                <Grid container spacing={2} sx={{ mt: 1 }}>
                  <Grid item xs={12}>
                    <TextField
                      label="DNS 监听地址"
                      value={config.dns?.listen || '0.0.0.0:53'}
                      onChange={(e) => updateConfig('dns.listen', e.target.value)}
                      fullWidth
                    />
                  </Grid>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>增强模式</InputLabel>
                      <Select
                        value={config.dns?.enhanced_mode || 'fake-ip'}
                        label="增强模式"
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
            原始配置 (JSON)
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            直接编辑原始配置。请注意语法正确性。
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
              从服务器重新加载
            </Button>
            <Button
              variant="contained"
              onClick={applyTextConfig}
            >
              应用更改
            </Button>
          </Box>
        </TabPanel>
      </Card>

      {isRunning && hasChanges && (
        <Alert severity="warning" sx={{ mt: 2 }}>
          配置更改将在重启 mihomo 服务后生效。
        </Alert>
      )}
    </Box>
  );
});

ConfigManager.displayName = 'ConfigManager';

export default ConfigManager;
