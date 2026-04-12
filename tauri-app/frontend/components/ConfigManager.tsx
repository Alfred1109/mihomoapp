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
  Dialog,
  DialogTitle,
  DialogContent,
  DialogContentText,
  DialogActions,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
} from '@mui/material';
import {
  Save,
  Refresh,
  Settings,
  NetworkCheck,
  Dns,
  Security,
  Code,
  History,
  Restore,
  Delete,
  Edit,
  Download,
  Upload,
  RestartAlt,
  CheckCircle,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import { useTranslation } from 'react-i18next';
import { TabPanel } from './common';
import type { ConfigValue } from '../types';

interface ConfigManagerProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const ConfigManager: React.FC<ConfigManagerProps> = React.memo(({ isRunning, showNotification }) => {
  const { t } = useTranslation();
  const [config, setConfig] = useState<ConfigValue | null>(null);
  const [loading, setLoading] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [configText, setConfigText] = useState('');
  const [tabValue, setTabValue] = useState(0);
  const [autostart, setAutostart] = useState(false);
  const [silentStart, setSilentStart] = useState(false);
  
  const [configBackups, setConfigBackups] = useState<string[]>([]);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState(false);
  const [deleteDialog, setDeleteDialog] = useState(false);
  const [editDialog, setEditDialog] = useState(false);
  const [resetDialog, setResetDialog] = useState(false);
  const [newLabel, setNewLabel] = useState('');

  const loadConfig = useCallback(async () => {
    setLoading(true);
    try {
      const configData = await invoke<ConfigValue>('get_mihomo_config');
      setConfig(configData);
      setConfigText(JSON.stringify(configData, null, 2));
      setHasChanges(false);
    } catch (error) {
      showNotification(`${t('config.loadFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showNotification, t]);

  const loadBackups = useCallback(async () => {
    try {
      const result = await invoke<string[]>('list_config_backups');
      setConfigBackups(result);
    } catch (error) {
      showNotification(`${t('config.loadBackupsFailed')}: ${error}`, 'error');
    }
  }, [showNotification, t]);

  const saveConfig = async () => {
    setLoading(true);
    try {
      await invoke('save_mihomo_config', { config });
      setHasChanges(false);
      showNotification(t('config.saveSuccess'), 'success');
      await loadBackups();
    } catch (error) {
      showNotification(`${t('config.saveFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleResetToDefault = async () => {
    setResetDialog(false);
    setLoading(true);
    try {
      const result = await invoke<string>('reset_config_to_default');
      await loadConfig();
      await loadBackups();
      showNotification(result, 'success');
    } catch (error) {
      showNotification(`${t('config.resetFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreBackup = async (backupFilename: string) => {
    setLoading(true);
    try {
      const result = await invoke<string>('restore_config_backup', { backup_filename: backupFilename });
      await loadConfig();
      showNotification(result, 'success');
      setConfirmDialog(false);
      setSelectedBackup(null);
    } catch (error) {
      showNotification(`${t('config.restoreFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteBackup = async (backupFilename: string) => {
    setLoading(true);
    try {
      const result = await invoke<string>('delete_config_backup', { backup_filename: backupFilename });
      showNotification(result, 'success');
      setDeleteDialog(false);
      setSelectedBackup(null);
      await loadBackups();
    } catch (error) {
      showNotification(`${t('config.deleteFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRenameBackup = async (oldFilename: string, label: string) => {
    if (!label.trim()) {
      showNotification(t('config.labelEmpty'), 'warning');
      return;
    }
    setLoading(true);
    try {
      await invoke<string>('rename_config_backup', { old_filename: oldFilename, new_label: label });
      showNotification(t('config.renameSuccess'), 'success');
      setEditDialog(false);
      setSelectedBackup(null);
      setNewLabel('');
      await loadBackups();
    } catch (error) {
      showNotification(`${t('config.renameFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleExportConfig = async () => {
    setLoading(true);
    try {
      const content = await invoke<string>('export_base_config');
      const filePath = await save({
        defaultPath: 'base_config.yaml',
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (filePath) {
        await writeTextFile(filePath, content);
        showNotification(t('config.exportSuccess'), 'success');
      }
    } catch (error) {
      showNotification(`${t('config.exportFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleImportConfig = async () => {
    setLoading(true);
    try {
      const filePath = await open({
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (filePath && typeof filePath === 'string') {
        const content = await readTextFile(filePath);
        await invoke<string>('import_base_config', { yaml_content: content });
        await loadConfig();
        showNotification(t('config.importSuccess'), 'success');
      }
    } catch (error) {
      showNotification(`${t('config.importFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const updateConfig = (field: string, value: unknown) => {
    if (!config) return;
    const newConfig = { ...config } as Record<string, unknown>;
    const fields = field.split('.');
    let current: Record<string, unknown> = newConfig;
    for (let i = 0; i < fields.length - 1; i++) {
      if (!current[fields[i]]) {
        current[fields[i]] = {};
      }
      current = current[fields[i]] as Record<string, unknown>;
    }
    current[fields[fields.length - 1]] = value;
    setConfig(newConfig as ConfigValue);
    setHasChanges(true);
  };

  const handleTabChange = (_: React.SyntheticEvent, newValue: number) => {
    setTabValue(newValue);
    if (newValue === 4) {
      loadBackups();
    }
  };

  const applyTextConfig = () => {
    try {
      const parsedConfig = JSON.parse(configText);
      setConfig(parsedConfig);
      setHasChanges(true);
      showNotification(t('config.loadedFromEditor'), 'success');
    } catch {
      showNotification(t('config.invalidJson'), 'error');
    }
  };

  const loadAppSettings = async () => {
    try {
      const autostartStatus = await invoke<boolean>('get_autostart_status');
      setAutostart(autostartStatus);
      const silentStatus = await invoke<boolean>('get_silent_start_status');
      setSilentStart(silentStatus);
    } catch {
    }
  };

  const handleAutostartChange = async (checked: boolean) => {
    try {
      await invoke('set_autostart', { enable: checked });
      setAutostart(checked);
      showNotification(checked ? t('config.autostartEnabled') : t('config.autostartDisabled'), 'success');
    } catch (error) {
      showNotification(`${t('config.autostartFailed')}: ${error}`, 'error');
    }
  };

  const handleSilentStartChange = async (checked: boolean) => {
    try {
      await invoke('set_silent_start', { enable: checked });
      setSilentStart(checked);
      showNotification(checked ? t('config.silentStartEnabled') : t('config.silentStartDisabled'), 'success');
    } catch (error) {
      showNotification(`${t('config.silentStartFailed')}: ${error}`, 'error');
    }
  };

  const formatBackupName = (filename: string): { date: string; time: string } => {
    const match = filename.match(/(\d{8})_(\d{6})/);
    if (match) {
      const dateStr = match[1];
      const timeStr = match[2];
      const date = `${dateStr.slice(0, 4)}-${dateStr.slice(4, 6)}-${dateStr.slice(6, 8)}`;
      const time = `${timeStr.slice(0, 2)}:${timeStr.slice(2, 4)}:${timeStr.slice(4, 6)}`;
      return { date, time };
    }
    return { date: '未知', time: '' };
  };

  const getBackupAge = (filename: string): string => {
    const match = filename.match(/(\d{8})_(\d{6})/);
    if (match) {
      const dateStr = match[1];
      const timeStr = match[2];
      const backupDate = new Date(
        parseInt(dateStr.slice(0, 4)),
        parseInt(dateStr.slice(4, 6)) - 1,
        parseInt(dateStr.slice(6, 8)),
        parseInt(timeStr.slice(0, 2)),
        parseInt(timeStr.slice(2, 4)),
        parseInt(timeStr.slice(4, 6))
      );
      const now = new Date();
      const diffMs = now.getTime() - backupDate.getTime();
      const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
      const diffDays = Math.floor(diffHours / 24);
      if (diffDays > 0) return `${diffDays}天前`;
      if (diffHours > 0) return `${diffHours}小时前`;
      return '刚刚';
    }
    return '';
  };

  useEffect(() => {
    loadConfig();
    loadAppSettings();
  }, [loadConfig]);

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
          <Typography variant="h6" gutterBottom>加载配置中...</Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">配置管理</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          {hasChanges && <Chip label="未保存的更改" color="warning" size="small" />}
          <Button variant="outlined" startIcon={<Refresh />} onClick={loadConfig} disabled={loading}>
            重新加载
          </Button>
          <Button variant="contained" startIcon={<Save />} onClick={saveConfig} disabled={loading || !hasChanges}>
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
            <Tab icon={<History />} label="备份与恢复" />
          </Tabs>
        </Box>

        <TabPanel value={tabValue} index={0}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>基本设置</Typography>
              <Grid container spacing={2}>
                <Grid item xs={6}>
                  <TextField label="HTTP 端口" type="number" value={config.port || 7890}
                    onChange={(e) => updateConfig('port', parseInt(e.target.value))} fullWidth />
                </Grid>
                <Grid item xs={6}>
                  <TextField label="SOCKS 端口" type="number" value={config['socks-port'] || 7891}
                    onChange={(e) => updateConfig('socks-port', parseInt(e.target.value))} fullWidth />
                </Grid>
                <Grid item xs={6}>
                  <TextField label="混合端口" type="number" value={config['mixed-port'] || 7890}
                    onChange={(e) => updateConfig('mixed-port', parseInt(e.target.value))} fullWidth />
                </Grid>
                <Grid item xs={6}>
                  <TextField label="外部控制器" value={config['external-controller'] || '127.0.0.1:9090'}
                    onChange={(e) => updateConfig('external-controller', e.target.value)} fullWidth />
                </Grid>
              </Grid>
              <Box sx={{ mt: 3 }}>
                <FormControlLabel control={<Switch checked={config['allow-lan'] || false}
                  onChange={(e) => updateConfig('allow-lan', e.target.checked)} />} label="允许局域网访问" />
                <Typography variant="body2" color="text.secondary">允许网络中的其他设备使用此代理</Typography>
              </Box>
              <Box sx={{ mt: 2 }}>
                <FormControlLabel control={<Switch checked={config.ipv6 !== false}
                  onChange={(e) => updateConfig('ipv6', e.target.checked)} />} label="启用 IPv6" />
                <Typography variant="body2" color="text.secondary">启用 IPv6 流量处理</Typography>
              </Box>
              <Divider sx={{ my: 3 }} />
              <Typography variant="h6" gutterBottom>应用设置</Typography>
              <Box sx={{ mt: 2 }}>
                <FormControlLabel control={<Switch checked={autostart}
                  onChange={(e) => handleAutostartChange(e.target.checked)} />} label="开机自启" />
                <Typography variant="body2" color="text.secondary">系统启动时自动运行 Mihomo Manager</Typography>
              </Box>
              <Box sx={{ mt: 2 }}>
                <FormControlLabel control={<Switch checked={silentStart}
                  onChange={(e) => handleSilentStartChange(e.target.checked)} />} label="静默启动" />
                <Typography variant="body2" color="text.secondary">启动时不显示窗口，仅在系统托盘显示图标</Typography>
              </Box>
            </Grid>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>TUN 模式</Typography>
              <FormControlLabel control={<Switch checked={config.tun?.enable || false}
                onChange={(e) => updateConfig('tun.enable', e.target.checked)} />} label="启用 TUN 模式" />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>系统级透明代理（需要管理员权限）</Typography>
              {config.tun?.enable && (
                <Grid container spacing={2}>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>TUN 协议栈</InputLabel>
                      <Select value={config.tun?.stack || 'system'} label="TUN 协议栈"
                        onChange={(e) => updateConfig('tun.stack', e.target.value)}>
                        <MenuItem value="system">System</MenuItem>
                        <MenuItem value="gvisor">gVisor</MenuItem>
                        <MenuItem value="lwip">LWIP</MenuItem>
                      </Select>
                    </FormControl>
                  </Grid>
                  <Grid item xs={12}>
                    <TextField label="设备名称" value={config.tun?.['device-name'] || ''}
                      onChange={(e) => updateConfig('tun.device-name', e.target.value)} placeholder="auto" fullWidth />
                  </Grid>
                </Grid>
              )}
            </Grid>
          </Grid>
        </TabPanel>

        <TabPanel value={tabValue} index={1}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>代理设置</Typography>
              <Grid container spacing={2}>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>代理模式</InputLabel>
                    <Select value={config.mode || 'rule'} label="代理模式"
                      onChange={(e) => updateConfig('mode', e.target.value)}>
                      <MenuItem value="rule">规则模式</MenuItem>
                      <MenuItem value="global">全局模式</MenuItem>
                      <MenuItem value="direct">直连模式</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12}>
                  <FormControl fullWidth>
                    <InputLabel>日志级别</InputLabel>
                    <Select value={config['log-level'] || 'info'} label="日志级别"
                      onChange={(e) => updateConfig('log-level', e.target.value)}>
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

        <TabPanel value={tabValue} index={2}>
          <Grid container spacing={3}>
            <Grid item xs={12} md={6}>
              <Typography variant="h6" gutterBottom>DNS 设置</Typography>
              <FormControlLabel control={<Switch checked={config.dns?.enable !== false}
                onChange={(e) => updateConfig('dns.enable', e.target.checked)} />} label="启用 DNS" />
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>使用 mihomo 的内置 DNS 服务器</Typography>
              <Box sx={{ mt: 2 }}>
                <FormControlLabel control={<Switch checked={config.dns?.ipv6 === true}
                  onChange={(e) => updateConfig('dns.ipv6', e.target.checked)} />} label="启用 IPv6" />
                <Typography variant="body2" color="text.secondary">启用 IPv6 DNS 解析</Typography>
              </Box>
              {config.dns?.enable !== false && (
                <Grid container spacing={2} sx={{ mt: 1 }}>
                  <Grid item xs={12}>
                    <TextField label="DNS 监听地址" value={config.dns?.listen || '0.0.0.0:53'}
                      onChange={(e) => updateConfig('dns.listen', e.target.value)} fullWidth />
                  </Grid>
                  <Grid item xs={12}>
                    <FormControl fullWidth>
                      <InputLabel>增强模式</InputLabel>
                      <Select value={config.dns?.enhanced_mode || 'fake-ip'} label="增强模式"
                        onChange={(e) => updateConfig('dns.enhanced_mode', e.target.value)}>
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

        <TabPanel value={tabValue} index={3}>
          <Typography variant="h6" gutterBottom>原始配置 (JSON)</Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>直接编辑原始配置。请注意语法正确性。</Typography>
          <Paper sx={{ p: 2, mb: 2 }}>
            <TextField multiline rows={20} value={configText} onChange={(e) => setConfigText(e.target.value)}
              fullWidth variant="outlined" sx={{ fontFamily: 'monospace' }} />
          </Paper>
          <Box sx={{ display: 'flex', gap: 2 }}>
            <Button variant="outlined" onClick={loadConfig}>从服务器重新加载</Button>
            <Button variant="contained" onClick={applyTextConfig}>应用更改</Button>
          </Box>
        </TabPanel>

        <TabPanel value={tabValue} index={4}>
          <Alert severity="info" sx={{ mb: 3 }}>
            <strong>架构说明：</strong>配置分为「基础配置」（DNS、TUN、规则等）和「订阅链接」两部分。
            恢复默认配置只会重置基础配置，不会影响订阅链接。
          </Alert>

          <Card variant="outlined" sx={{ mb: 3 }}>
            <CardContent>
              <Typography variant="h6" gutterBottom>配置操作</Typography>
              <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
                <Button variant="outlined" startIcon={<Download />} onClick={handleExportConfig} disabled={loading}>
                  导出配置
                </Button>
                <Button variant="outlined" startIcon={<Upload />} onClick={handleImportConfig} disabled={loading}>
                  导入配置
                </Button>
                <Button variant="outlined" color="warning" startIcon={<RestartAlt />}
                  onClick={() => setResetDialog(true)} disabled={loading}>
                  恢复默认配置
                </Button>
              </Box>
            </CardContent>
          </Card>

          <Card variant="outlined">
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                <Typography variant="h6">配置备份 ({configBackups.length})</Typography>
                <Button size="small" startIcon={<Refresh />} onClick={loadBackups} disabled={loading}>刷新</Button>
              </Box>
              {configBackups.length === 0 ? (
                <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', py: 4 }}>
                  暂无配置备份（保存配置时自动创建）
                </Typography>
              ) : (
                <List>
                  {configBackups.map((backup, index) => {
                    const { date, time } = formatBackupName(backup);
                    const age = getBackupAge(backup);
                    const isLatest = index === 0;
                    return (
                      <ListItem key={backup} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 1, mb: 1,
                        bgcolor: isLatest ? 'action.hover' : 'background.paper' }}>
                        <ListItemText
                          primary={
                            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                              <Typography variant="subtitle1">{date} {time}</Typography>
                              {isLatest && <Chip icon={<CheckCircle />} label="最新" size="small" color="success" />}
                            </Box>
                          }
                          secondary={<Typography variant="caption" color="text.secondary">{age}</Typography>}
                        />
                        <ListItemSecondaryAction>
                          <Box sx={{ display: 'flex', gap: 0.5 }}>
                            <IconButton size="small" onClick={() => { setSelectedBackup(backup); setNewLabel(''); setEditDialog(true); }}
                              disabled={loading} title="编辑标签"><Edit fontSize="small" /></IconButton>
                            <IconButton size="small" color="primary" onClick={() => { setSelectedBackup(backup); setConfirmDialog(true); }}
                              disabled={loading} title="恢复"><Restore fontSize="small" /></IconButton>
                            <IconButton size="small" color="error" onClick={() => { setSelectedBackup(backup); setDeleteDialog(true); }}
                              disabled={loading} title="删除"><Delete fontSize="small" /></IconButton>
                          </Box>
                        </ListItemSecondaryAction>
                      </ListItem>
                    );
                  })}
                </List>
              )}
            </CardContent>
          </Card>
        </TabPanel>
      </Card>

      {isRunning && hasChanges && (
        <Alert severity="warning" sx={{ mt: 2 }}>配置更改将在重启 mihomo 服务后生效。</Alert>
      )}

      <Dialog open={confirmDialog} onClose={() => setConfirmDialog(false)}>
        <DialogTitle>确认恢复配置备份</DialogTitle>
        <DialogContent>
          <DialogContentText>确定要恢复此备份吗？当前配置将被覆盖。</DialogContentText>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">{formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}</Typography>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDialog(false)}>取消</Button>
          <Button variant="contained" color="warning" onClick={() => selectedBackup && handleRestoreBackup(selectedBackup)} disabled={loading}>
            确认恢复
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={deleteDialog} onClose={() => setDeleteDialog(false)}>
        <DialogTitle>确认删除备份</DialogTitle>
        <DialogContent>
          <DialogContentText>确定要删除此备份吗？此操作无法撤销！</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialog(false)}>取消</Button>
          <Button variant="contained" color="error" onClick={() => selectedBackup && handleDeleteBackup(selectedBackup)} disabled={loading}>
            确认删除
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={editDialog} onClose={() => setEditDialog(false)}>
        <DialogTitle>编辑备份标签</DialogTitle>
        <DialogContent>
          <TextField fullWidth value={newLabel} onChange={(e) => setNewLabel(e.target.value)}
            placeholder="输入标签，例如：更新前备份" size="small" sx={{ mt: 1 }} />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialog(false)}>取消</Button>
          <Button variant="contained" onClick={() => selectedBackup && handleRenameBackup(selectedBackup, newLabel)}
            disabled={loading || !newLabel.trim()}>保存</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={resetDialog} onClose={() => setResetDialog(false)}>
        <DialogTitle>确认恢复默认配置</DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>此操作将把配置重置为默认值，不会影响订阅链接。</Alert>
          <DialogContentText>当前配置将被覆盖，确定继续吗？</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setResetDialog(false)}>取消</Button>
          <Button variant="contained" color="warning" onClick={handleResetToDefault} disabled={loading}>确认恢复默认</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
});

ConfigManager.displayName = 'ConfigManager';

export default ConfigManager;
