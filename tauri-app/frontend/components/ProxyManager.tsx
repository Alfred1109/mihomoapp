import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  LinearProgress,
  Alert,
  TableSortLabel,
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
  history?: Array<{ delay: number; time: string }>;
}

interface ProxyGroup {
  name: string;
  type: string;
  now?: string;
  all: string[];
  history?: Array<{ name: string; delay: number; time: string }>;
}

type SortField = 'name' | 'type' | 'delay' | 'status';
type SortOrder = 'asc' | 'desc';

const ProxyManager: React.FC<ProxyManagerProps> = React.memo(({ isRunning, showNotification }) => {
  const [proxies, setProxies] = useState<{ [key: string]: ProxyNode | ProxyGroup }>({});
  const [groups, setGroups] = useState<ProxyGroup[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [sortField, setSortField] = useState<SortField>('name');
  const [sortOrder, setSortOrder] = useState<SortOrder>('asc');
  const [proxyHistory, setProxyHistory] = useState<Array<{ groupName: string; nodeName: string; time: string }>>([]);

  const extractProxyGroups = (response: any) => {
    // Extract proxy groups, PROXY组优先
    // 过滤掉mihomo内置的冗余组和auto组（auto组通过PROXY组选择即可）
    const excludeBuiltinGroups = ['GLOBAL', 'COMPATIBLE', 'PASS', 'DIRECT', 'REJECT', 'REJECT-DROP', 'auto'];
    const proxyGroups: ProxyGroup[] = [];
    Object.entries(response.proxies || {}).forEach(([name, proxy]: [string, any]) => {
      if ((proxy.type === 'Selector' || proxy.type === 'URLTest' || proxy.type === 'Fallback' || proxy.type === 'LoadBalance') 
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
    // 优先选中PROXY组，如果不存在则选第一个可用组
    if (proxyGroups.length > 0) {
      const hasProxy = proxyGroups.some(g => g.name === 'PROXY');
      setSelectedGroup(hasProxy ? 'PROXY' : proxyGroups[0].name);
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
      // 等待一小段时间确保Mihomo内部状态已更新
      await new Promise(resolve => setTimeout(resolve, 500));
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
      
      // 记录切换历史，保留最近3条
      const newHistory = [
        { groupName, nodeName: proxyName, time: new Date().toISOString() },
        ...proxyHistory.filter(h => !(h.groupName === groupName && h.nodeName === proxyName))
      ].slice(0, 3);
      setProxyHistory(newHistory);
      
      // 保存到localStorage
      try {
        localStorage.setItem('proxyHistory', JSON.stringify(newHistory));
      } catch (e) {
        console.error('Failed to save proxy history:', e);
      }
      
      showNotification('代理切换成功', 'success');
      await loadProxies(); // Refresh data
    } catch (error) {
      showNotification(`切换代理失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const formatDelay = (delay?: number): string => {
    if (delay === undefined || delay === null) return 'N/A';
    if (delay <= 0) return 'Timeout';
    return `${delay}ms`;
  };

  const getDelayColor = (delay?: number): 'success' | 'warning' | 'error' | 'default' => {
    if (delay === undefined || delay === null || delay <= 0) return 'default';
    if (delay < 100) return 'success';
    if (delay < 300) return 'warning';
    return 'error';
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      // 切换排序顺序
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      // 新字段，默认升序
      setSortField(field);
      setSortOrder('asc');
    }
  };

  const sortNodes = (nodeNames: string[]) => {
    return [...nodeNames].sort((a, b) => {
      const nodeA = proxies[a] as any;
      const nodeB = proxies[b] as any;
      
      let compareResult = 0;
      
      switch (sortField) {
        case 'name':
          compareResult = a.localeCompare(b);
          break;
        case 'type':
          compareResult = (nodeA?.type || '').localeCompare(nodeB?.type || '');
          break;
        case 'delay':
          // 使用最新的测速记录（Mihomo的history是倒序的，最新在索引0）
          const historyA = nodeA?.history;
          const historyB = nodeB?.history;
          const delayA = (historyA && historyA.length > 0) ? historyA[0]?.delay : 999999;
          const delayB = (historyB && historyB.length > 0) ? historyB[0]?.delay : 999999;
          compareResult = (delayA || 999999) - (delayB || 999999);
          break;
        case 'status':
          const statusA = nodeA?.alive === true ? 1 : 0;
          const statusB = nodeB?.alive === true ? 1 : 0;
          compareResult = statusB - statusA; // 在线的排前面
          break;
      }
      
      return sortOrder === 'asc' ? compareResult : -compareResult;
    });
  };

  useEffect(() => {
    loadProxies();
    
    // 从localStorage加载历史记录
    try {
      const saved = localStorage.getItem('proxyHistory');
      if (saved) {
        setProxyHistory(JSON.parse(saved));
      }
    } catch (e) {
      console.error('Failed to load proxy history:', e);
    }
    
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
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'name'}
                            direction={sortField === 'name' ? sortOrder : 'asc'}
                            onClick={() => handleSort('name')}
                          >
                            节点名称
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'type'}
                            direction={sortField === 'type' ? sortOrder : 'asc'}
                            onClick={() => handleSort('type')}
                          >
                            类型
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'delay'}
                            direction={sortField === 'delay' ? sortOrder : 'asc'}
                            onClick={() => handleSort('delay')}
                          >
                            延迟
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'status'}
                            direction={sortField === 'status' ? sortOrder : 'asc'}
                            onClick={() => handleSort('status')}
                          >
                            状态
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>操作</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {sortNodes(group.all).map((nodeName) => {
                        const node = proxies[nodeName] as any;
                        const isActive = group.now === nodeName;
                        // 从节点自身的history字段获取最新延迟（Mihomo的history是倒序的，最新在索引0）
                        const history = node?.history;
                        const nodeDelay = (history && history.length > 0) ? history[0]?.delay : undefined;
                        
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
                              {(() => {
                                // 如果有有效的延迟数据，说明节点可用
                                const hasValidDelay = nodeDelay !== undefined && nodeDelay > 0;
                                const isOnline = node?.alive === true || hasValidDelay;
                                return (
                                  <Chip
                                    icon={isOnline ? <CheckCircle /> : <Error />}
                                    label={isOnline ? '在线' : '离线'}
                                    size="small"
                                    color={isOnline ? 'success' : 'error'}
                                  />
                                );
                              })()}
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
      {selectedGroup && proxyHistory.length > 0 && (() => {
        // 只显示当前选中组的历史记录
        const groupHistory = proxyHistory.filter(h => h.groupName === selectedGroup).slice(0, 3);
        
        if (groupHistory.length === 0) return null;

        return (
          <Card sx={{ mt: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                历史使用节点 - {selectedGroup}
              </Typography>
              
              <TableContainer component={Paper}>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>节点名称</TableCell>
                      <TableCell>当前延迟</TableCell>
                      <TableCell>切换时间</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {groupHistory.map((item, index) => {
                      const node = proxies[item.nodeName] as any;
                      const history = node?.history;
                      const nodeDelay = (history && history.length > 0) ? history[0]?.delay : undefined;
                      
                      return (
                        <TableRow key={index}>
                          <TableCell>{item.nodeName}</TableCell>
                          <TableCell>
                            <Chip
                              label={formatDelay(nodeDelay)}
                              size="small"
                              color={getDelayColor(nodeDelay)}
                            />
                          </TableCell>
                          <TableCell>
                            {new Date(item.time).toLocaleString('zh-CN', {
                              month: '2-digit',
                              day: '2-digit',
                              hour: '2-digit',
                              minute: '2-digit'
                            })}
                          </TableCell>
                        </TableRow>
                      );
                    })}
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
});

ProxyManager.displayName = 'ProxyManager';

export default ProxyManager;
