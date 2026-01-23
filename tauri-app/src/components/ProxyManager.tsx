import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Grid,
  Button,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  IconButton,
  LinearProgress,
  Alert,
} from '@mui/material';
import {
  Refresh,
  Speed,
  Router,
  CheckCircle,
  Error,
  SignalWifi4Bar,
  SignalWifiOff,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { proxyCache, defer } from '../utils/performance';

interface ProxyManagerProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

interface ProxyNode {
  name: string;
  type: string;
  delay?: number;
  alive: boolean;
}

interface ProxyGroup {
  name: string;
  type: string;
  now?: string;
  all: string[];
  history?: Array<{ name: string; delay: number; time: string }>;
}

const ProxyManager: React.FC<ProxyManagerProps> = ({ isRunning, showNotification }) => {
  const [proxies, setProxies] = useState<{ [key: string]: ProxyNode | ProxyGroup }>({});
  const [groups, setGroups] = useState<ProxyGroup[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<string>('');
  const [loading, setLoading] = useState(false);

  const extractProxyGroups = (response: any) => {
    // Extract proxy groups, PROXY组优先
    // 过滤掉mihomo内置的冗余组和auto组（auto组通过PROXY组选择即可）
    const excludeBuiltinGroups = ['GLOBAL', 'COMPATIBLE', 'PASS', 'DIRECT', 'REJECT', 'REJECT-DROP', 'auto'];
    const proxyGroups: ProxyGroup[] = [];
    Object.entries(response.proxies || {}).forEach(([name, proxy]: [string, any]) => {
      if ((proxy.type === 'Selector' || proxy.type === 'URLTest' || proxy.type === 'Fallback') 
          && !excludeBuiltinGroups.includes(name)) {
        proxyGroups.push({
          name,
          type: proxy.type,
          now: proxy.now,
          all: proxy.all || [],
          history: proxy.history || []
        });
      }
    });
    
    // 将PROXY组排在最前面
    proxyGroups.sort((a, b) => {
      if (a.name === 'PROXY') return -1;
      if (b.name === 'PROXY') return 1;
      return 0;
    });
    
    setGroups(proxyGroups);
    // 始终选中PROXY组（因为移除了卡片选择）
    if (proxyGroups.length > 0) {
      setSelectedGroup('PROXY');
    }
  };

  const loadProxies = async () => {
    if (!isRunning) return;
    
    // 检查缓存
    const cached = proxyCache.get('proxies');
    if (cached) {
      setProxies(cached.proxies || {});
      // 从缓存中提取代理组
      extractProxyGroups(cached);
      // 使用缓存数据后，异步更新
      defer(async () => {
        try {
          const response = await invoke<any>('get_proxies');
          proxyCache.set('proxies', response);
          setProxies(response.proxies || {});
          extractProxyGroups(response);
        } catch (error) {
          console.error('Background proxy update failed:', error);
        }
      }, 100);
      return;
    }
    
    setLoading(true);
    try {
      const response = await invoke<any>('get_proxies');
      proxyCache.set('proxies', response);
      setProxies(response.proxies || {});
      extractProxyGroups(response);
    } catch (error) {
      showNotification(`加载代理失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const testGroupDelay = async (groupName: string) => {
    try {
      setLoading(true);
      await invoke('test_group_delay', { groupName });
      // Reload proxies to get updated delay info
      await loadProxies();
      showNotification(`延迟测试完成: ${groupName}`, 'success');
    } catch (error) {
      console.error(`Failed to test delay for group ${groupName}:`, error);
      showNotification(`延迟测试失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleTestAllProxies = async () => {
    try {
      setLoading(true);
      showNotification('开始批量测速，请稍候...', 'info');
      const result = await invoke<any>('test_all_proxies', { 
        testUrl: 'http://1.1.1.1',
        timeout: 5000 
      });
      // 直接刷新列表，不弹窗
      await loadProxies();
      showNotification(`批量测速完成！成功测试 ${result.success}/${result.total} 个节点`, 'success');
    } catch (error) {
      showNotification(`批量测速失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleSwitchProxy = async (groupName: string, proxyName: string) => {
    try {
      setLoading(true);
      await invoke('switch_proxy', { groupName, proxyName });
      showNotification('代理切换成功', 'success');
      await loadProxies(); // Refresh data
    } catch (error) {
      showNotification(`切换代理失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const formatDelay = (delay?: number): string => {
    if (!delay) return 'N/A';
    if (delay < 0) return 'Timeout';
    return `${delay}ms`;
  };

  const getDelayColor = (delay?: number): 'success' | 'warning' | 'error' | 'default' => {
    if (!delay || delay < 0) return 'default';
    if (delay < 100) return 'success';
    if (delay < 300) return 'warning';
    return 'error';
  };

  useEffect(() => {
    loadProxies();
    
    if (isRunning) {
      // 延长刷新间隔到30秒，减少不必要的API调用
      const interval = setInterval(loadProxies, 30000);
      return () => clearInterval(interval);
    }
  }, [isRunning]);

  if (!isRunning) {
    return (
      <Card>
        <CardContent sx={{ textAlign: 'center', py: 8 }}>
          <SignalWifiOff sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
          <Typography variant="h6" gutterBottom>
            代理服务未运行
          </Typography>
          <Typography variant="body2" color="text.secondary">
            请启动 Mihomo 服务以管理代理节点和组
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">代理管理</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<Speed />}
            onClick={handleTestAllProxies}
            disabled={!isRunning || loading}
          >
            批量测速
          </Button>
          <Button
            variant="contained"
            startIcon={<Refresh />}
            onClick={loadProxies}
            disabled={!isRunning || loading}
          >
            刷新
          </Button>
        </Box>
      </Box>

      {loading && <LinearProgress sx={{ mb: 2 }} />}

      {/* PROXY组状态信息 */}
      {groups.length > 0 && groups[0].name === 'PROXY' && (
        <Alert severity="info" sx={{ mb: 3 }}>
          <Box>
            <Typography variant="subtitle1" sx={{ fontWeight: 'bold', mb: 1 }}>
              PROXY 主代理组
            </Typography>
            <Typography variant="body2" sx={{ mb: 0.5 }}>
              • 当前选择：<strong>{groups[0].now || '无'}</strong>
              {groups[0].now === 'auto' && ' - 自动测速并选择延迟最低的节点'}
              {groups[0].now !== 'auto' && groups[0].now && groups[0].now !== 'DIRECT' && ' - 固定使用此节点'}
              {groups[0].now === 'DIRECT' && ' - 直连模式（不使用代理）'}
            </Typography>
            <Typography variant="body2" sx={{ mb: 0.5 }}>
              • 可用节点：{groups[0].all?.length || 0} 个
              {groups[0].now !== 'auto' && ' | 如需自动选择最快节点，请在下方选择 "auto"'}
              {groups[0].now === 'auto' && ' | 如需固定使用某个节点，请在下方直接选择'}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              💡 提示：所有流量都通过PROXY组，选择合适的节点以获得最佳体验
            </Typography>
          </Box>
        </Alert>
      )}

      {/* Detailed Proxy Management */}
      {groups.length > 0 && (
        <Card>
          <CardContent>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
              <Typography variant="h6">
                节点列表
              </Typography>
              {selectedGroup === 'PROXY' && groups.find(g => g.name === 'PROXY')?.now === 'auto' && (
                <Chip 
                  label="自动模式：选择延迟最低的节点" 
                  color="success" 
                  size="small" 
                  icon={<Speed />}
                />
              )}
            </Box>

            {/* Node List for Selected Group */}
            {selectedGroup && (() => {
              const group = groups.find(g => g.name === selectedGroup);
              if (!group || !group.all) return null;

              return (
                <TableContainer component={Paper}>
                  <Table>
                    <TableHead>
                      <TableRow>
                        <TableCell>节点名称</TableCell>
                        <TableCell>类型</TableCell>
                        <TableCell>延迟</TableCell>
                        <TableCell>状态</TableCell>
                        <TableCell>操作</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {group.all.map((nodeName) => {
                        const node = proxies[nodeName] as any;
                        const isActive = group.now === nodeName;
                        // 从节点自身的history字段获取最新延迟
                        const nodeDelay = node?.history?.[0]?.delay;
                        
                        return (
                          <TableRow 
                            key={nodeName}
                            sx={{ backgroundColor: isActive ? 'action.selected' : 'inherit' }}
                          >
                            <TableCell>
                              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                                {isActive ? (
                                  <SignalWifi4Bar sx={{ fontSize: 16, color: 'success.main' }} />
                                ) : (
                                  <Router sx={{ fontSize: 16, color: 'text.disabled' }} />
                                )}
                                <Typography variant="subtitle2">{nodeName}</Typography>
                              </Box>
                            </TableCell>
                            <TableCell>
                              <Chip label={node?.type || 'Unknown'} size="small" variant="outlined" />
                            </TableCell>
                            <TableCell>
                              <Chip
                                label={formatDelay(nodeDelay)}
                                size="small"
                                color={getDelayColor(nodeDelay)}
                              />
                            </TableCell>
                            <TableCell>
                              <Chip
                                icon={node?.alive === true ? <CheckCircle /> : <Error />}
                                label={node?.alive === true ? '在线' : (nodeDelay ? '未知' : '离线')}
                                size="small"
                                color={node?.alive === true ? 'success' : (nodeDelay ? 'warning' : 'error')}
                              />
                            </TableCell>
                            <TableCell>
                              <Button
                                variant={isActive ? "outlined" : "contained"}
                                size="small"
                                onClick={() => handleSwitchProxy(selectedGroup, nodeName)}
                                disabled={loading || isActive}
                              >
                                {isActive ? '当前' : '切换'}
                              </Button>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                </TableContainer>
              );
            })()}
          </CardContent>
        </Card>
      )}

      {/* Connection History */}
      {selectedGroup && (() => {
        const group = groups.find(g => g.name === selectedGroup);
        if (!group?.history?.length) return null;

        return (
          <Card sx={{ mt: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                Connection History for {selectedGroup}
              </Typography>
              
              <TableContainer component={Paper}>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>Node</TableCell>
                      <TableCell>Delay</TableCell>
                      <TableCell>Time</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {group.history.slice(0, 10).map((item, index) => (
                      <TableRow key={index}>
                        <TableCell>{item.name}</TableCell>
                        <TableCell>
                          <Chip
                            label={formatDelay(item.delay)}
                            size="small"
                            color={getDelayColor(item.delay)}
                          />
                        </TableCell>
                        <TableCell>
                          {new Date(item.time).toLocaleTimeString()}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </CardContent>
          </Card>
        );
      })()}

      {/* No Proxies Available */}
      {groups.length === 0 && !loading && (
        <Card>
          <CardContent sx={{ textAlign: 'center', py: 8 }}>
            <Speed sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" gutterBottom>
              暂无代理组
            </Typography>
            <Typography variant="body2" color="text.secondary">
              请先配置代理订阅以管理代理节点
            </Typography>
          </CardContent>
        </Card>
      )}
    </Box>
  );
};

export default ProxyManager;
