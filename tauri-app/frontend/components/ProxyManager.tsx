import React, { useState, useEffect, useCallback } from 'react';
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
import { useTranslation } from 'react-i18next';
import { proxyCache, defer } from '../utils/performance';
import type { ProxyNode, ProxyGroup, ProxiesResponse } from '../types';

interface ProxyManagerProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

type SortField = 'name' | 'type' | 'delay' | 'status';
type SortOrder = 'asc' | 'desc';

const ProxyManager: React.FC<ProxyManagerProps> = React.memo(({ isRunning, showNotification }) => {
  const { t } = useTranslation();
  const [proxies, setProxies] = useState<{ [key: string]: ProxyNode | ProxyGroup }>({});
  const [groups, setGroups] = useState<ProxyGroup[]>([]);
  const [selectedGroup, setSelectedGroup] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [sortField, setSortField] = useState<SortField>('name');
  const [sortOrder, setSortOrder] = useState<SortOrder>('asc');
  const [proxyHistory, setProxyHistory] = useState<Array<{ groupName: string; nodeName: string; time: string }>>([]);

  const extractProxyGroups = (response: ProxiesResponse) => {
    const excludeBuiltinGroups = ['GLOBAL', 'COMPATIBLE', 'PASS', 'DIRECT', 'REJECT', 'REJECT-DROP', 'auto'];
    const proxyGroups: ProxyGroup[] = [];
    Object.entries(response.proxies || {}).forEach(([name, proxy]) => {
      const proxyData = proxy as ProxyGroup;
      if ((proxyData.type === 'Selector' || proxyData.type === 'URLTest' || proxyData.type === 'Fallback' || proxyData.type === 'LoadBalance') 
          && !excludeBuiltinGroups.includes(name)) {
        proxyGroups.push({
          name,
          type: proxyData.type,
          now: proxyData.now,
          all: proxyData.all || [],
          history: proxyData.history || []
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
    
    const cached = proxyCache.get('proxies');
    if (cached) {
      setProxies(cached.proxies || {});
      extractProxyGroups(cached);
      defer(async () => {
        try {
          const response = await invoke<ProxiesResponse>('get_proxies');
          proxyCache.set('proxies', response);
          setProxies(response.proxies || {});
          extractProxyGroups(response);
        } catch {
        }
      }, 100);
      return;
    }
    
    setLoading(true);
    try {
      const response = await invoke<ProxiesResponse>('get_proxies');
      proxyCache.set('proxies', response);
      setProxies(response.proxies || {});
      extractProxyGroups(response);
    } catch (error) {
      showNotification(`${t('proxy.loadFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleTestAllProxies = async () => {
    try {
      setLoading(true);
      showNotification(t('proxy.testStarting'), 'info');
      const result = await invoke<{ success: number; total: number }>('test_all_proxies', { 
        test_url: 'http://1.1.1.1',
        timeout: 5000 
      });
      await loadProxies();
      showNotification(t('proxy.testComplete', { success: result.success, total: result.total }), 'success');
    } catch (error) {
      showNotification(`${t('proxy.testFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleSwitchProxy = async (groupName: string, proxyName: string) => {
    try {
      setLoading(true);
      await invoke('switch_proxy', { groupName, proxyName });
      
      const newHistory = [
        { groupName, nodeName: proxyName, time: new Date().toISOString() },
        ...proxyHistory.filter(h => !(h.groupName === groupName && h.nodeName === proxyName))
      ].slice(0, 3);
      setProxyHistory(newHistory);
      
      try {
        localStorage.setItem('proxyHistory', JSON.stringify(newHistory));
      } catch {
      }
      
      showNotification(t('proxy.switchSuccess'), 'success');
      await loadProxies();
    } catch (error) {
      showNotification(`${t('proxy.switchFailed')}: ${error}`, 'error');
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
      const nodeA = proxies[a] as ProxyNode;
      const nodeB = proxies[b] as ProxyNode;
      
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
    
    try {
      const saved = localStorage.getItem('proxyHistory');
      if (saved) {
        setProxyHistory(JSON.parse(saved));
      }
    } catch {
    }
    
    if (isRunning) {
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
            {t('proxy.notRunning')}
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {t('proxy.notRunningDesc')}
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">{t('proxy.title')}</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<Speed />}
            onClick={handleTestAllProxies}
            disabled={!isRunning || loading}
          >
            {t('proxy.testAll')}
          </Button>
          <Button
            variant="contained"
            startIcon={<Refresh />}
            onClick={loadProxies}
            disabled={!isRunning || loading}
          >
            {t('proxy.refresh')}
          </Button>
        </Box>
      </Box>

      {loading && <LinearProgress sx={{ mb: 2 }} />}

      {groups.length > 0 && groups[0].name === 'PROXY' && (
        <Alert severity="info" sx={{ mb: 3 }}>
          <Box>
            <Typography variant="subtitle1" sx={{ fontWeight: 'bold', mb: 1 }}>
              {t('proxy.proxyGroupInfo')}
            </Typography>
            <Typography variant="body2" sx={{ mb: 0.5 }}>
              • {t('proxy.currentSelection')}: <strong>{groups[0].now || 'N/A'}</strong>
              {groups[0].now === 'auto' && ` - ${t('proxy.autoDesc')}`}
              {groups[0].now !== 'auto' && groups[0].now && groups[0].now !== 'DIRECT' && ` - ${t('proxy.fixedDesc')}`}
              {groups[0].now === 'DIRECT' && ` - ${t('proxy.directDesc')}`}
            </Typography>
            <Typography variant="body2" sx={{ mb: 0.5 }}>
              • {t('proxy.availableCount')}: {groups[0].all?.length || 0}
              {groups[0].now !== 'auto' && ` | ${t('proxy.selectAutoTip')}`}
              {groups[0].now === 'auto' && ` | ${t('proxy.selectManualTip')}`}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {t('proxy.tip')}
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
                {t('proxy.nodeList')}
              </Typography>
              {selectedGroup === 'PROXY' && groups.find(g => g.name === 'PROXY')?.now === 'auto' && (
                <Chip 
                  label={t('proxy.autoMode')} 
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
                            {t('proxy.nodeName')}
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'type'}
                            direction={sortField === 'type' ? sortOrder : 'asc'}
                            onClick={() => handleSort('type')}
                          >
                            {t('proxy.type')}
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'delay'}
                            direction={sortField === 'delay' ? sortOrder : 'asc'}
                            onClick={() => handleSort('delay')}
                          >
                            {t('proxy.delay')}
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>
                          <TableSortLabel
                            active={sortField === 'status'}
                            direction={sortField === 'status' ? sortOrder : 'asc'}
                            onClick={() => handleSort('status')}
                          >
                            {t('proxy.status')}
                          </TableSortLabel>
                        </TableCell>
                        <TableCell>{t('proxy.action')}</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {sortNodes(group.all).map((nodeName) => {
                        const node = proxies[nodeName] as ProxyNode;
                        const isActive = group.now === nodeName;
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
                                const hasValidDelay = nodeDelay !== undefined && nodeDelay > 0;
                                const isOnline = node?.alive === true || hasValidDelay;
                                return (
                                  <Chip
                                    icon={isOnline ? <CheckCircle /> : <Error />}
                                    label={isOnline ? t('proxy.online') : t('proxy.offline')}
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
                                {isActive ? t('proxy.current') : t('proxy.switch')}
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

      {selectedGroup && proxyHistory.length > 0 && (() => {
        const groupHistory = proxyHistory.filter(h => h.groupName === selectedGroup).slice(0, 3);
        
        if (groupHistory.length === 0) return null;

        return (
          <Card sx={{ mt: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>
                {t('proxy.historyTitle')} - {selectedGroup}
              </Typography>
              
              <TableContainer component={Paper}>
                <Table size="small">
                  <TableHead>
                    <TableRow>
                      <TableCell>{t('proxy.nodeName')}</TableCell>
                      <TableCell>{t('proxy.currentDelay')}</TableCell>
                      <TableCell>{t('proxy.switchTime')}</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {groupHistory.map((item, index) => {
                      const node = proxies[item.nodeName] as ProxyNode;
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
                            {new Date(item.time).toLocaleString(undefined, {
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

      {groups.length === 0 && !loading && (
        <Card>
          <CardContent sx={{ textAlign: 'center', py: 8 }}>
            <Speed sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
            <Typography variant="h6" gutterBottom>
              {t('proxy.noProxyGroups')}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {t('proxy.noProxyGroupsDesc')}
            </Typography>
          </CardContent>
        </Card>
      )}
    </Box>
  );
});

ProxyManager.displayName = 'ProxyManager';

export default ProxyManager;
