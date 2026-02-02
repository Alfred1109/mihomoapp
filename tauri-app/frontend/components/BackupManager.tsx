import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Chip,
  Alert,
  Tabs,
  Tab,
  Divider,
  TextField,
} from '@mui/material';
import {
  Restore,
  Delete,
  Refresh,
  History,
  CheckCircle,
  Edit,
  Download,
  Upload,
  RestartAlt,
  Link as LinkIcon,
  Settings,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import { TabPanel } from './common';

interface BackupManagerProps {
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const BackupManager: React.FC<BackupManagerProps> = React.memo(({ showNotification }) => {
  const [tabValue, setTabValue] = useState(0);
  const [configBackups, setConfigBackups] = useState<string[]>([]);
  const [subscriptionBackups, setSubscriptionBackups] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState(false);
  const [deleteDialog, setDeleteDialog] = useState(false);
  const [editDialog, setEditDialog] = useState(false);
  const [resetDialog, setResetDialog] = useState(false);
  const [subscriptionRestoreDialog, setSubscriptionRestoreDialog] = useState(false);
  const [newLabel, setNewLabel] = useState('');

  const loadAllBackups = useCallback(async () => {
    setLoading(true);
    try {
      const [configResult, subResult] = await Promise.all([
        invoke<string[]>('list_config_backups'),
        invoke<string[]>('list_subscription_backups'),
      ]);
      setConfigBackups(configResult);
      setSubscriptionBackups(subResult);
    } catch (error) {
      showNotification(`加载备份列表失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showNotification]);

  useEffect(() => {
    loadAllBackups();
  }, [loadAllBackups]);

  const handleRestoreConfig = async (backupFilename: string) => {
    setLoading(true);
    try {
      const result = await invoke<string>('restore_config_backup', { backupFilename });
      showNotification(result, 'success');
      setConfirmDialog(false);
      setSelectedBackup(null);
    } catch (error) {
      showNotification(`恢复备份失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteConfig = async (backupFilename: string) => {
    setLoading(true);
    try {
      const result = await invoke<string>('delete_config_backup', { backupFilename });
      showNotification(result, 'success');
      setDeleteDialog(false);
      setSelectedBackup(null);
      await loadAllBackups();
    } catch (error) {
      showNotification(`删除备份失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRename = async (oldFilename: string, label: string) => {
    if (!label.trim()) {
      showNotification('标签不能为空', 'warning');
      return;
    }
    setLoading(true);
    try {
      await invoke<string>('rename_config_backup', { oldFilename, newLabel: label });
      showNotification('备份已重命名', 'success');
      setEditDialog(false);
      setSelectedBackup(null);
      setNewLabel('');
      await loadAllBackups();
    } catch (error) {
      showNotification(`重命名备份失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleResetToDefault = async () => {
    setLoading(true);
    try {
      const result = await invoke<string>('reset_config_to_default');
      showNotification(result, 'success');
      setResetDialog(false);
    } catch (error) {
      showNotification(`恢复默认配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleExportSubscriptions = async () => {
    setLoading(true);
    try {
      const content = await invoke<string>('export_subscriptions');
      const filePath = await save({
        defaultPath: 'subscriptions.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (filePath) {
        await writeTextFile(filePath, content);
        showNotification('订阅链接导出成功', 'success');
      }
    } catch (error) {
      showNotification(`导出订阅链接失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleImportSubscriptions = async () => {
    setLoading(true);
    try {
      const filePath = await open({
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (filePath && typeof filePath === 'string') {
        const content = await readTextFile(filePath);
        const count = await invoke<number>('import_subscriptions', { jsonContent: content });
        showNotification(`成功导入 ${count} 个订阅链接`, 'success');
        await loadAllBackups();
      }
    } catch (error) {
      showNotification(`导入订阅链接失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleBackupSubscriptions = async () => {
    setLoading(true);
    try {
      const filename = await invoke<string>('backup_subscriptions');
      showNotification(`订阅链接已备份: ${filename}`, 'success');
      await loadAllBackups();
    } catch (error) {
      showNotification(`备份订阅链接失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreSubscriptions = async (backupFilename: string) => {
    setLoading(true);
    try {
      const count = await invoke<number>('restore_subscriptions_from_backup', { backupFilename });
      showNotification(`成功恢复 ${count} 个订阅链接`, 'success');
      setSubscriptionRestoreDialog(false);
      setSelectedBackup(null);
    } catch (error) {
      showNotification(`恢复订阅链接失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleExportBaseConfig = async () => {
    setLoading(true);
    try {
      const content = await invoke<string>('export_base_config');
      const filePath = await save({
        defaultPath: 'base_config.yaml',
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (filePath) {
        await writeTextFile(filePath, content);
        showNotification('基础配置导出成功', 'success');
      }
    } catch (error) {
      showNotification(`导出基础配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleImportBaseConfig = async () => {
    setLoading(true);
    try {
      const filePath = await open({
        filters: [{ name: 'YAML', extensions: ['yaml', 'yml'] }],
      });
      if (filePath && typeof filePath === 'string') {
        const content = await readTextFile(filePath);
        await invoke<string>('import_base_config', { yamlContent: content });
        showNotification('基础配置导入成功，请更新订阅以应用更改', 'success');
      }
    } catch (error) {
      showNotification(`导入基础配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRegenerateConfig = async () => {
    setLoading(true);
    try {
      const result = await invoke<string>('regenerate_runtime_config');
      showNotification(result, 'success');
    } catch (error) {
      showNotification(`重新生成配置失败: ${error}`, 'error');
    } finally {
      setLoading(false);
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

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">配置与备份管理</Typography>
        <Button
          variant="contained"
          startIcon={<Refresh />}
          onClick={loadAllBackups}
          disabled={loading}
        >
          刷新
        </Button>
      </Box>

      <Alert severity="info" sx={{ mb: 3 }}>
        <strong>架构说明：</strong>配置分为「基础配置」（DNS、TUN、规则等）和「订阅链接」两部分。
        恢复默认配置只会重置基础配置，不会影响订阅链接。换电脑时只需导入订阅链接即可。
      </Alert>

      <Tabs value={tabValue} onChange={(_, v) => setTabValue(v)} sx={{ mb: 2 }}>
        <Tab icon={<Settings />} label="基础配置" iconPosition="start" />
        <Tab icon={<LinkIcon />} label="订阅链接" iconPosition="start" />
        <Tab icon={<History />} label="运行时配置备份" iconPosition="start" />
      </Tabs>

      <TabPanel value={tabValue} index={0}>
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="h6" gutterBottom>基础配置管理</Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              基础配置包含 DNS、TUN、路由规则等设置，不包含代理节点。
            </Typography>
            <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
              <Button
                variant="outlined"
                startIcon={<Download />}
                onClick={handleExportBaseConfig}
                disabled={loading}
              >
                导出基础配置
              </Button>
              <Button
                variant="outlined"
                startIcon={<Upload />}
                onClick={handleImportBaseConfig}
                disabled={loading}
              >
                导入基础配置
              </Button>
              <Button
                variant="outlined"
                color="warning"
                startIcon={<RestartAlt />}
                onClick={() => setResetDialog(true)}
                disabled={loading}
              >
                恢复默认配置
              </Button>
              <Button
                variant="contained"
                startIcon={<Refresh />}
                onClick={handleRegenerateConfig}
                disabled={loading}
              >
                重新生成运行时配置
              </Button>
            </Box>
          </CardContent>
        </Card>
      </TabPanel>

      <TabPanel value={tabValue} index={1}>
        <Card sx={{ mb: 3 }}>
          <CardContent>
            <Typography variant="h6" gutterBottom>订阅链接管理</Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              订阅链接与基础配置分离存储，可独立导出/导入/备份。
            </Typography>
            <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', mb: 3 }}>
              <Button
                variant="outlined"
                startIcon={<Download />}
                onClick={handleExportSubscriptions}
                disabled={loading}
              >
                导出订阅链接
              </Button>
              <Button
                variant="outlined"
                startIcon={<Upload />}
                onClick={handleImportSubscriptions}
                disabled={loading}
              >
                导入订阅链接
              </Button>
              <Button
                variant="contained"
                startIcon={<History />}
                onClick={handleBackupSubscriptions}
                disabled={loading}
              >
                创建订阅备份
              </Button>
            </Box>

            <Divider sx={{ my: 2 }} />

            <Typography variant="subtitle1" sx={{ mb: 2 }}>
              订阅链接备份 ({subscriptionBackups.length})
            </Typography>

            {subscriptionBackups.length === 0 ? (
              <Typography variant="body2" color="text.secondary" sx={{ textAlign: 'center', py: 2 }}>
                暂无订阅链接备份
              </Typography>
            ) : (
              <List>
                {subscriptionBackups.map((backup, index) => {
                  const { date, time } = formatBackupName(backup);
                  const age = getBackupAge(backup);
                  const isLatest = index === 0;

                  return (
                    <ListItem
                      key={backup}
                      sx={{
                        border: '1px solid',
                        borderColor: 'divider',
                        borderRadius: 1,
                        mb: 1,
                        bgcolor: isLatest ? 'action.hover' : 'background.paper',
                      }}
                    >
                      <ListItemText
                        primary={
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                            <Typography variant="subtitle2">{date} {time}</Typography>
                            {isLatest && (
                              <Chip icon={<CheckCircle />} label="最新" size="small" color="success" />
                            )}
                          </Box>
                        }
                        secondary={age}
                      />
                      <ListItemSecondaryAction>
                        <IconButton
                          onClick={() => {
                            setSelectedBackup(backup);
                            setSubscriptionRestoreDialog(true);
                          }}
                          disabled={loading}
                          title="恢复此订阅备份"
                          size="small"
                          color="primary"
                        >
                          <Restore fontSize="small" />
                        </IconButton>
                      </ListItemSecondaryAction>
                    </ListItem>
                  );
                })}
              </List>
            )}
          </CardContent>
        </Card>
      </TabPanel>

      <TabPanel value={tabValue} index={2}>
        <Card>
          <CardContent>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
              <History />
              <Typography variant="h6">运行时配置备份 ({configBackups.length})</Typography>
            </Box>

            <Alert severity="info" sx={{ mb: 2 }}>
              运行时配置备份包含完整配置（基础配置+代理节点），每次配置更改时自动创建。
            </Alert>

            {configBackups.length === 0 ? (
              <Box sx={{ textAlign: 'center', py: 4 }}>
                <Typography variant="body2" color="text.secondary">暂无配置备份</Typography>
              </Box>
            ) : (
              <List>
                {configBackups.map((backup, index) => {
                  const { date, time } = formatBackupName(backup);
                  const age = getBackupAge(backup);
                  const isLatest = index === 0;

                  return (
                    <ListItem
                      key={backup}
                      sx={{
                        border: '1px solid',
                        borderColor: 'divider',
                        borderRadius: 1,
                        mb: 1,
                        bgcolor: isLatest ? 'action.hover' : 'background.paper',
                      }}
                    >
                      <ListItemText
                        primary={
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                            <Typography variant="subtitle1">{date} {time}</Typography>
                            {isLatest && (
                              <Chip icon={<CheckCircle />} label="最新" size="small" color="success" />
                            )}
                          </Box>
                        }
                        secondary={
                          <Box sx={{ display: 'flex', gap: 2, mt: 0.5 }}>
                            <Typography variant="caption" color="text.secondary">{age}</Typography>
                            <Typography variant="caption" color="text.secondary">{backup}</Typography>
                          </Box>
                        }
                      />
                      <ListItemSecondaryAction>
                        <Box sx={{ display: 'flex', gap: 1 }}>
                          <IconButton
                            onClick={() => {
                              setSelectedBackup(backup);
                              setNewLabel('');
                              setEditDialog(true);
                            }}
                            disabled={loading}
                            title="编辑备份标签"
                            size="small"
                          >
                            <Edit fontSize="small" />
                          </IconButton>
                          <IconButton
                            onClick={() => {
                              setSelectedBackup(backup);
                              setConfirmDialog(true);
                            }}
                            disabled={loading}
                            title="恢复此备份"
                            size="small"
                            color="primary"
                          >
                            <Restore fontSize="small" />
                          </IconButton>
                          <IconButton
                            onClick={() => {
                              setSelectedBackup(backup);
                              setDeleteDialog(true);
                            }}
                            disabled={loading}
                            title="删除此备份"
                            size="small"
                            color="error"
                          >
                            <Delete fontSize="small" />
                          </IconButton>
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

      <Dialog open={confirmDialog} onClose={() => setConfirmDialog(false)}>
        <DialogTitle>确认恢复配置备份</DialogTitle>
        <DialogContent>
          <Typography>确定要恢复以下备份吗？</Typography>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
            </Box>
          )}
          <Alert severity="warning" sx={{ mt: 2 }}>
            当前配置将被覆盖，请确保已保存重要更改！
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDialog(false)}>取消</Button>
          <Button
            variant="contained"
            color="warning"
            onClick={() => selectedBackup && handleRestoreConfig(selectedBackup)}
            disabled={loading}
          >
            确认恢复
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={deleteDialog} onClose={() => setDeleteDialog(false)}>
        <DialogTitle>确认删除备份</DialogTitle>
        <DialogContent>
          <Typography>确定要删除以下备份吗？此操作无法撤销！</Typography>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialog(false)}>取消</Button>
          <Button
            variant="contained"
            color="error"
            onClick={() => selectedBackup && handleDeleteConfig(selectedBackup)}
            disabled={loading}
          >
            确认删除
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={editDialog} onClose={() => setEditDialog(false)}>
        <DialogTitle>编辑备份标签</DialogTitle>
        <DialogContent>
          <Typography variant="body2" sx={{ mb: 2 }}>为备份添加自定义标签：</Typography>
          <TextField
            fullWidth
            value={newLabel}
            onChange={(e) => setNewLabel(e.target.value)}
            placeholder="输入标签，例如：更新前备份"
            size="small"
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialog(false)}>取消</Button>
          <Button
            variant="contained"
            onClick={() => selectedBackup && handleRename(selectedBackup, newLabel)}
            disabled={loading || !newLabel.trim()}
          >
            保存
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={resetDialog} onClose={() => setResetDialog(false)}>
        <DialogTitle>确认恢复默认配置</DialogTitle>
        <DialogContent>
          <Alert severity="warning" sx={{ mb: 2 }}>
            此操作将把基础配置（DNS、TUN、路由规则等）重置为默认值。
          </Alert>
          <Typography variant="body2">
            <strong>不会</strong>影响您的订阅链接。如果有活动的订阅，将自动使用默认基础配置重新生成运行时配置。
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setResetDialog(false)}>取消</Button>
          <Button
            variant="contained"
            color="warning"
            onClick={handleResetToDefault}
            disabled={loading}
          >
            确认恢复默认
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={subscriptionRestoreDialog} onClose={() => setSubscriptionRestoreDialog(false)}>
        <DialogTitle>确认恢复订阅链接</DialogTitle>
        <DialogContent>
          <Typography>确定要恢复以下订阅备份吗？</Typography>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
            </Box>
          )}
          <Alert severity="info" sx={{ mt: 2 }}>
            恢复订阅链接后，请手动更新订阅以获取最新的代理节点。
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSubscriptionRestoreDialog(false)}>取消</Button>
          <Button
            variant="contained"
            onClick={() => selectedBackup && handleRestoreSubscriptions(selectedBackup)}
            disabled={loading}
          >
            确认恢复
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
});

BackupManager.displayName = 'BackupManager';

export default BackupManager;
