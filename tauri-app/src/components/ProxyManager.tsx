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

  const loadProxies = async () => {
    if (!isRunning) return;
    
    setLoading(true);
    try {
      const response = await invoke<any>('get_proxies');
      setProxies(response.proxies || {});
      
      // Extract proxy groups
      const proxyGroups: ProxyGroup[] = [];
      Object.entries(response.proxies || {}).forEach(([name, proxy]: [string, any]) => {
        if (proxy.type === 'Selector' || proxy.type === 'URLTest' || proxy.type === 'Fallback') {
          proxyGroups.push({
            name,
            type: proxy.type,
            now: proxy.now,
            all: proxy.all || [],
            history: proxy.history || []
          });
        }
      });
      
      setGroups(proxyGroups);
      if (proxyGroups.length > 0 && !selectedGroup) {
        setSelectedGroup(proxyGroups[0].name);
      }
    } catch (error) {
      showNotification(`Failed to load proxies: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleSwitchProxy = async (groupName: string, proxyName: string) => {
    try {
      setLoading(true);
      await invoke('switch_proxy', { groupName, proxyName });
      showNotification('Proxy switched successfully', 'success');
      await loadProxies(); // Refresh data
    } catch (error) {
      showNotification(`Failed to switch proxy: ${error}`, 'error');
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
      const interval = setInterval(loadProxies, 10000); // Refresh every 10 seconds
      return () => clearInterval(interval);
    }
  }, [isRunning]);

  if (!isRunning) {
    return (
      <Card>
        <CardContent sx={{ textAlign: 'center', py: 8 }}>
          <SignalWifiOff sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
          <Typography variant="h6" gutterBottom>
            Proxy Service Not Running
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Start the mihomo service to manage proxy nodes and groups
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">Proxy Manager</Typography>
        <Button
          variant="outlined"
          startIcon={<Refresh />}
          onClick={loadProxies}
          disabled={loading}
        >
          Refresh
        </Button>
      </Box>

      {loading && <LinearProgress sx={{ mb: 2 }} />}

      {/* Proxy Groups Overview */}
      <Grid container spacing={3} sx={{ mb: 3 }}>
        {groups.map((group) => (
          <Grid item xs={12} md={6} lg={4} key={group.name}>
            <Card>
              <CardContent>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
                  <Typography variant="h6">{group.name}</Typography>
                  <Chip label={group.type} size="small" variant="outlined" />
                </Box>
                <Typography variant="body2" color="text.secondary" gutterBottom>
                  Current: {group.now || 'None'}
                </Typography>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mt: 2 }}>
                  <Typography variant="body2" color="text.secondary">
                    Available Nodes: {group.all.length}
                  </Typography>
                  {group.now && (
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <CheckCircle sx={{ fontSize: 16, color: 'success.main' }} />
                      <Typography variant="body2">Active</Typography>
                    </Box>
                  )}
                </Box>
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>

      {/* Detailed Proxy Management */}
      {groups.length > 0 && (
        <Card>
          <CardContent>
            <Typography variant="h6" gutterBottom>
              Node Management
            </Typography>
            
            {/* Group Selector */}
            <FormControl sx={{ minWidth: 200, mb: 3 }}>
              <InputLabel>Proxy Group</InputLabel>
              <Select
                value={selectedGroup}
                label="Proxy Group"
                onChange={(e) => setSelectedGroup(e.target.value)}
              >
                {groups.map((group) => (
                  <MenuItem key={group.name} value={group.name}>
                    {group.name} ({group.type})
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            {/* Node List for Selected Group */}
            {selectedGroup && (() => {
              const group = groups.find(g => g.name === selectedGroup);
              if (!group || !group.all) return null;

              return (
                <TableContainer component={Paper}>
                  <Table>
                    <TableHead>
                      <TableRow>
                        <TableCell>Node Name</TableCell>
                        <TableCell>Type</TableCell>
                        <TableCell>Status</TableCell>
                        <TableCell>Delay</TableCell>
                        <TableCell>Action</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {group.all.map((nodeName) => {
                        const node = proxies[nodeName] as ProxyNode;
                        const isActive = group.now === nodeName;
                        const history = group.history?.find(h => h.name === nodeName);
                        
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
                                icon={node?.alive !== false ? <CheckCircle /> : <Error />}
                                label={node?.alive !== false ? 'Online' : 'Offline'}
                                size="small"
                                color={node?.alive !== false ? 'success' : 'error'}
                              />
                            </TableCell>
                            <TableCell>
                              <Chip
                                label={formatDelay(history?.delay || node?.delay)}
                                size="small"
                                color={getDelayColor(history?.delay || node?.delay)}
                              />
                            </TableCell>
                            <TableCell>
                              <Button
                                variant={isActive ? "outlined" : "contained"}
                                size="small"
                                onClick={() => handleSwitchProxy(selectedGroup, nodeName)}
                                disabled={loading || isActive}
                              >
                                {isActive ? 'Current' : 'Switch'}
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
              No Proxy Groups Available
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Configure proxy subscriptions first to manage proxy nodes
            </Typography>
          </CardContent>
        </Card>
      )}
    </Box>
  );
};

export default ProxyManager;
