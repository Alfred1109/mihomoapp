import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  Grid,
  Switch,
  FormControlLabel,
  Chip,
  LinearProgress,
  Alert,
  List,
  ListItem,
  ListItemText,
  Divider,
  ButtonGroup,
  CircularProgress,
  Table,
  TableCell,
  TableBody,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  IconButton,
} from '@mui/material';
import {
  PlayArrow,
  Stop,
  Refresh,
  NetworkCheck,
  Speed,
  CloudDownload,
  CloudUpload,
  Router,
  Security,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface DashboardProps {
  isRunning: boolean;
  onStatusChange: (status: boolean) => void;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

interface TrafficStats {
  up: number;
  down: number;
}

interface ProxyInfo {
  name: string;
  type: string;
  delay: number | null;
  alive: boolean;
}

const Dashboard: React.FC<DashboardProps> = ({ isRunning, onStatusChange, showNotification }) => {
  const [loading, setLoading] = useState(false);
  const [trafficStats, setTrafficStats] = useState<TrafficStats>({ up: 0, down: 0 });
  const [proxies, setProxies] = useState<ProxyInfo[]>([]);
  const [config, setConfig] = useState<any>(null);
  const [tunMode, setTunMode] = useState(false);
  const [serviceStatus, setServiceStatus] = useState<string>('not_installed');
  const [serviceLoading, setServiceLoading] = useState(false);
  const [ipInfo, setIpInfo] = useState<any>(null);
  const [ipLoading, setIpLoading] = useState(false);

  const handleStartService = async () => {
    if (typeof window === 'undefined' || !window.__TAURI_IPC__) {
      showNotification('Tauri API not available in browser mode', 'warning');
      return;
    }
    
    setLoading(true);
    try {
      const result = await invoke<string>('start_mihomo_service');
      showNotification(result, 'success');
      onStatusChange(true);
    } catch (error) {
      showNotification(`Failed to start service: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleStopService = async () => {
    if (typeof window === 'undefined' || !window.__TAURI_IPC__) {
      showNotification('Tauri API not available in browser mode', 'warning');
      return;
    }
    
    setLoading(true);
    try {
      const result = await invoke<string>('stop_mihomo_service');
      showNotification(result, 'success');
      onStatusChange(false);
    } catch (error) {
      showNotification(`Failed to stop service: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleStop = async () => {
    setLoading(true);
    try {
      await invoke('stop_mihomo');
      showNotification('Mihomo stopped successfully', 'success');
      onStatusChange(false);
    } catch (error) {
      showNotification(`Failed to stop Mihomo: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRestart = async () => {
    setLoading(true);
    try {
      await invoke('stop_mihomo');
      // 等待服务完全停止
      await new Promise(resolve => setTimeout(resolve, 1000));
      await invoke('start_mihomo');
      showNotification('Mihomo重启成功，配置已应用', 'success');
      onStatusChange(true);
      // 重新加载数据
      setTimeout(() => {
        loadDashboardData();
        loadIpInfo(true);
      }, 1000);
    } catch (error) {
      showNotification(`Failed to restart Mihomo: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const loadDashboardData = async () => {
    if (typeof window === 'undefined' || !window.__TAURI_IPC__) return;

    try {
      // Load proxies only when service is running
      if (isRunning) {
        const proxiesData = await invoke<any>('get_proxies');
        if (proxiesData.proxies) {
          const proxyList: ProxyInfo[] = Object.entries(proxiesData.proxies)
            .filter(([name, proxy]: [string, any]) => {
              // 只显示实际的代理节点，过滤掉代理组和特殊节点
              const excludeTypes = ['Selector', 'URLTest', 'Fallback', 'LoadBalance', 'Relay'];
              const excludeNames = ['DIRECT', 'REJECT', 'COMPATIBLE', 'PASS', 'REJECT-DROP'];
              return !excludeTypes.includes(proxy.type) && !excludeNames.includes(name);
            })
            .map(([name, proxy]: [string, any]) => ({
              name,
              type: proxy.type,
              delay: proxy.history?.[0]?.delay || null,
              alive: proxy.alive !== false,
            }))
            .slice(0, 5); // Show only first 5 proxies
          setProxies(proxyList);
        }
      }

      // Always load config so TUN toggle persists across tabs
      const configData = await invoke<any>('get_mihomo_config');
      setConfig(configData);
      setTunMode(configData.tun?.enable || false);
    } catch (error) {
      console.error('Failed to load dashboard data:', error);
    }
  };

  const handleTunToggle = async (enable: boolean) => {
    try {
      await invoke<string>('enable_tun_mode', { enable });
      setTunMode(enable);
      showNotification(`TUN mode ${enable ? 'enabled' : 'disabled'}`, 'success');
    } catch (error) {
      showNotification(`Failed to ${enable ? 'enable' : 'disable'} TUN mode: ${error}`, 'error');
    }
  };

  const checkServiceStatus = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    try {
      const status = await invoke<string>('get_mihomo_service_status');
      setServiceStatus(status);
    } catch (error) {
      console.error('Failed to check service status:', error);
    }
  };

  const checkBundledMihomo = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    try {
      const path = await invoke<string>('get_bundled_mihomo_path');
      return true;
    } catch (error) {
      console.error('内置 mihomo 检查失败:', error);
      return false;
    }
  };

  const handleInstallService = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
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

  const handleStartServiceCmd = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    setServiceLoading(true);
    try {
      const result = await invoke<string>('start_mihomo_service_cmd');
      showNotification(result, 'success');
      await checkServiceStatus();
      onStatusChange(true);
    } catch (error) {
      showNotification(`启动失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const handleStopServiceCmd = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    setServiceLoading(true);
    try {
      const result = await invoke<string>('stop_mihomo_service_cmd');
      showNotification(result, 'success');
      await checkServiceStatus();
      onStatusChange(false);
    } catch (error) {
      showNotification(`停止失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const handleUninstallService = async () => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    setServiceLoading(true);
    try {
      const result = await invoke<string>('uninstall_mihomo_service');
      showNotification(result, 'success');
      await checkServiceStatus();
      onStatusChange(false);
    } catch (error) {
      showNotification(`卸载失败: ${error}`, 'error');
    } finally {
      setServiceLoading(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    if (bytes === 0) return '0 B';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${sizes[i]}`;
  };

  const loadIpInfo = async (forceRefresh = false) => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    // 检查缓存（5分钟内不重复请求）
    const cachedData = localStorage.getItem('ipInfo');
    const cachedTime = localStorage.getItem('ipInfoTime');
    
    if (!forceRefresh && cachedData && cachedTime) {
      const timeDiff = Date.now() - parseInt(cachedTime);
      if (timeDiff < 5 * 60 * 1000) { // 5分钟
        setIpInfo(JSON.parse(cachedData));
        return;
      }
    }
    
    setIpLoading(true);
    try {
      const data = await invoke<any>('get_current_ip');
      setIpInfo(data);
      // 缓存数据
      localStorage.setItem('ipInfo', JSON.stringify(data));
      localStorage.setItem('ipInfoTime', Date.now().toString());
    } catch (error) {
      console.error('Failed to get IP info:', error);
      // 如果请求失败，尝试使用缓存数据
      if (cachedData) {
        setIpInfo(JSON.parse(cachedData));
      }
    } finally {
      setIpLoading(false);
    }
  };

  useEffect(() => {
    loadDashboardData();
    checkServiceStatus();
    loadIpInfo();
    
    if (isRunning) {
      const interval = setInterval(loadDashboardData, 5000);
      return () => clearInterval(interval);
    }
  }, [isRunning]);

  return (
    <Box>
      {/* Service Control */}
      <Grid container spacing={3} sx={{ mb: 3 }}>
        <Grid item xs={12} md={8}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Mihomo 服务管理
              </Typography>
              
              {/* Service Status */}
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
                <Chip
                  icon={isRunning ? <PlayArrow /> : <Stop />}
                  label={isRunning ? 'Running' : 'Stopped'}
                  color={isRunning ? 'success' : 'error'}
                  variant="filled"
                />
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
                  variant="outlined"
                />
                {(loading || serviceLoading) && <CircularProgress size={20} />}
              </Box>

              {/* Service Management Buttons */}
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
                  <Button
                    variant="contained"
                    color="success"
                    onClick={handleStartServiceCmd}
                    disabled={serviceLoading}
                    startIcon={<PlayArrow />}
                  >
                    启动服务
                  </Button>
                )}

                {(serviceStatus === 'installed' || serviceStatus === 'stopped' || serviceStatus === 'running') && (
                  <Button
                    variant="outlined"
                    color="warning"
                    onClick={handleInstallService}
                    disabled={serviceLoading}
                  >
                    重新安装服务
                  </Button>
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
                      onClick={handleStopServiceCmd}
                      disabled={serviceLoading}
                      startIcon={<Stop />}
                    >
                      停止服务
                    </Button>
                  </>
                )}

                {(serviceStatus === 'installed' || serviceStatus === 'stopped' || serviceStatus === 'running') && (
                  <Button
                    variant="outlined"
                    color="error"
                    onClick={handleUninstallService}
                    disabled={serviceLoading}
                  >
                    卸载服务
                  </Button>
                )}

                <Button
                  variant="outlined"
                  onClick={checkServiceStatus}
                  disabled={serviceLoading}
                  startIcon={<Refresh />}
                >
                  刷新状态
                </Button>
              </Box>

            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={4}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <Box>
                  <Typography variant="h6" gutterBottom>
                    TUN Mode
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    System-wide transparent proxy
                  </Typography>
                </Box>
                <FormControlLabel
                  control={
                    <Switch
                      checked={tunMode}
                      onChange={(e) => handleTunToggle(e.target.checked)}
                      disabled={serviceStatus !== 'running'}
                    />
                  }
                  label=""
                />
              </Box>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Stats */}
      <Grid container spacing={3} sx={{ mb: 3 }}>
        <Grid item xs={6} md={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center' }}>
              <CloudUpload sx={{ fontSize: 40, color: 'primary.main', mb: 1 }} />
              <Typography variant="h6">{formatBytes(trafficStats.up)}</Typography>
              <Typography variant="body2" color="text.secondary">
                Upload
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={6} md={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center' }}>
              <CloudDownload sx={{ fontSize: 40, color: 'secondary.main', mb: 1 }} />
              <Typography variant="h6">{formatBytes(trafficStats.down)}</Typography>
              <Typography variant="body2" color="text.secondary">
                Download
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={6} md={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center' }}>
              <Router sx={{ fontSize: 40, color: 'success.main', mb: 1 }} />
              <Typography variant="h6">{proxies.length}</Typography>
              <Typography variant="body2" color="text.secondary">
                Active Proxies
              </Typography>
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={6} md={3}>
          <Card>
            <CardContent sx={{ textAlign: 'center' }}>
              <Security sx={{ fontSize: 40, color: 'warning.main', mb: 1 }} />
              <Typography variant="h6">{config?.mode || 'Rule'}</Typography>
              <Typography variant="body2" color="text.secondary">
                Proxy Mode
              </Typography>
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* IP Information */}
      <Grid container spacing={3} sx={{ mb: 3 }}>
        <Grid item xs={12}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                <Typography variant="h6">IP Information</Typography>
                <IconButton onClick={() => loadIpInfo(true)} disabled={ipLoading} size="small">
                  <Refresh />
                </IconButton>
              </Box>
              {ipLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', py: 3 }}>
                  <CircularProgress />
                </Box>
              ) : ipInfo ? (
                <Grid container spacing={2}>
                  <Grid item xs={12} sm={6} md={3}>
                    <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                      <Typography variant="body2" color="text.secondary" gutterBottom>
                        IP Address
                      </Typography>
                      <Typography variant="h6">
                        {ipInfo.query || ipInfo.ip || 'N/A'}
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={12} sm={6} md={3}>
                    <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                      <Typography variant="body2" color="text.secondary" gutterBottom>
                        Country
                      </Typography>
                      <Typography variant="h6">
                        {ipInfo.country || ipInfo.countryCode || 'N/A'}
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={12} sm={6} md={3}>
                    <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                      <Typography variant="body2" color="text.secondary" gutterBottom>
                        Region
                      </Typography>
                      <Typography variant="h6">
                        {ipInfo.regionName || ipInfo.region || 'N/A'}
                      </Typography>
                    </Box>
                  </Grid>
                  <Grid item xs={12} sm={6} md={3}>
                    <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                      <Typography variant="body2" color="text.secondary" gutterBottom>
                        City
                      </Typography>
                      <Typography variant="h6">
                        {ipInfo.city || 'N/A'}
                      </Typography>
                    </Box>
                  </Grid>
                  {ipInfo.isp && (
                    <Grid item xs={12} sm={6} md={4}>
                      <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                        <Typography variant="body2" color="text.secondary" gutterBottom>
                          ISP
                        </Typography>
                        <Typography variant="body1">
                          {ipInfo.isp}
                        </Typography>
                      </Box>
                    </Grid>
                  )}
                  {ipInfo.org && (
                    <Grid item xs={12} sm={6} md={4}>
                      <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                        <Typography variant="body2" color="text.secondary" gutterBottom>
                          Organization
                        </Typography>
                        <Typography variant="body1">
                          {ipInfo.org}
                        </Typography>
                      </Box>
                    </Grid>
                  )}
                  {ipInfo.timezone && (
                    <Grid item xs={12} sm={6} md={4}>
                      <Box sx={{ p: 2, bgcolor: 'background.default', borderRadius: 1 }}>
                        <Typography variant="body2" color="text.secondary" gutterBottom>
                          Timezone
                        </Typography>
                        <Typography variant="body1">
                          {ipInfo.timezone}
                        </Typography>
                      </Box>
                    </Grid>
                  )}
                </Grid>
              ) : (
                <Alert severity="info">
                  Click refresh to load IP information
                </Alert>
              )}
            </CardContent>
          </Card>
        </Grid>
      </Grid>

      {/* Recent Proxies */}
      <Grid container spacing={3}>
        <Grid item xs={12} md={8}>
          <Card>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                <Typography variant="h6">Active Proxies</Typography>
                <IconButton onClick={loadDashboardData} disabled={!isRunning}>
                  <Refresh />
                </IconButton>
              </Box>

              {proxies.length > 0 ? (
                <TableContainer>
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>Name</TableCell>
                        <TableCell>Type</TableCell>
                        <TableCell>Status</TableCell>
                        <TableCell>Delay</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {proxies.map((proxy) => (
                        <TableRow key={proxy.name}>
                          <TableCell>{proxy.name}</TableCell>
                          <TableCell>
                            <Chip label={proxy.type} size="small" variant="outlined" />
                          </TableCell>
                          <TableCell>
                            <Chip
                              label={proxy.alive ? 'Online' : 'Offline'}
                              size="small"
                              color={proxy.alive ? 'success' : 'error'}
                            />
                          </TableCell>
                          <TableCell>
                            {proxy.delay ? `${proxy.delay}ms` : 'N/A'}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </TableContainer>
              ) : (
                <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', py: 4 }}>
                  {isRunning ? 'No proxy data available' : 'Start the service to view proxy information'}
                </Typography>
              )}
            </CardContent>
          </Card>
        </Grid>

        <Grid item xs={12} md={4}>
          <Card>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Configuration
              </Typography>
              
              {config ? (
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Typography variant="body2">HTTP Port:</Typography>
                    <Typography variant="body2">{config.port || 7890}</Typography>
                  </Box>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Typography variant="body2">SOCKS Port:</Typography>
                    <Typography variant="body2">{config['socks-port'] || 7891}</Typography>
                  </Box>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Typography variant="body2">Allow LAN:</Typography>
                    <Chip
                      label={config['allow-lan'] ? 'Yes' : 'No'}
                      size="small"
                      color={config['allow-lan'] ? 'success' : 'default'}
                    />
                  </Box>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
                    <Typography variant="body2">Log Level:</Typography>
                    <Typography variant="body2">{config['log-level'] || 'info'}</Typography>
                  </Box>
                </Box>
              ) : (
                <Typography variant="body2" color="text.secondary">
                  Configuration not available
                </Typography>
              )}
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Box>
  );
};

export default Dashboard;
