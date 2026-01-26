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
} from '@mui/material';
import {
  Restore,
  Delete,
  Refresh,
  History,
  CheckCircle,
  Edit,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface BackupManagerProps {
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const BackupManager: React.FC<BackupManagerProps> = React.memo(({ showNotification }) => {
  const [backups, setBackups] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState(false);
  const [deleteDialog, setDeleteDialog] = useState(false);
  const [editDialog, setEditDialog] = useState(false);
  const [newLabel, setNewLabel] = useState('');

  useEffect(() => {
    loadBackups();
  }, []);

  const loadBackups = async () => {
    setLoading(true);
    try {
      const result = await invoke<string[]>('list_config_backups');
      setBackups(result);
    } catch (error) {
      showNotification(`加载备份列表失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRestore = async (backupFilename: string) => {
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

  const handleDelete = async (backupFilename: string) => {
    setLoading(true);
    try {
      const result = await invoke<string>('delete_config_backup', { backupFilename });
      showNotification(result, 'success');
      setDeleteDialog(false);
      setSelectedBackup(null);
      await loadBackups();
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
      await loadBackups();
    } catch (error) {
      showNotification(`重命名备份失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const formatBackupName = (filename: string): { date: string; time: string } => {
    // 格式: config.yaml.backup.20260123_063900
    const match = filename.match(/backup\.(\d{8})_(\d{6})/);
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
    const match = filename.match(/backup\.(\d{8})_(\d{6})/);
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

      if (diffDays > 0) {
        return `${diffDays}天前`;
      } else if (diffHours > 0) {
        return `${diffHours}小时前`;
      } else {
        return '刚刚';
      }
    }
    return '';
  };

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">配置备份管理</Typography>
        <Button
          variant="contained"
          startIcon={<Refresh />}
          onClick={loadBackups}
          disabled={loading}
        >
          刷新
        </Button>
      </Box>

      <Alert severity="info" sx={{ mb: 3 }}>
        <strong>提示：</strong>每次订阅更新前会自动备份配置，最多保留5个备份。
        恢复备份后需要重启mihomo服务以应用更改。
      </Alert>

      <Card>
        <CardContent>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
            <History />
            <Typography variant="h6">
              备份列表 ({backups.length}/5)
            </Typography>
          </Box>

          {backups.length === 0 ? (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Typography variant="body2" color="text.secondary">
                暂无配置备份
              </Typography>
              <Typography variant="caption" color="text.secondary">
                更新订阅时会自动创建备份
              </Typography>
            </Box>
          ) : (
            <List>
              {backups.map((backup, index) => {
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
                          <Typography variant="subtitle1">
                            {date} {time}
                          </Typography>
                          {isLatest && (
                            <Chip
                              icon={<CheckCircle />}
                              label="最新"
                              size="small"
                              color="success"
                            />
                          )}
                        </Box>
                      }
                      secondary={
                        <Box sx={{ display: 'flex', gap: 2, mt: 0.5 }}>
                          <Typography variant="caption" color="text.secondary">
                            {age}
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {backup}
                          </Typography>
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

      {/* 恢复确认对话框 */}
      <Dialog open={confirmDialog} onClose={() => setConfirmDialog(false)}>
        <DialogTitle>确认恢复备份</DialogTitle>
        <DialogContent>
          <Typography>
            确定要恢复以下备份吗？
          </Typography>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {selectedBackup}
              </Typography>
            </Box>
          )}
          <Alert severity="warning" sx={{ mt: 2 }}>
            当前配置将被覆盖，请确保已保存重要更改！
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDialog(false)}>
            取消
          </Button>
          <Button
            variant="contained"
            color="warning"
            onClick={() => selectedBackup && handleRestore(selectedBackup)}
            disabled={loading}
          >
            确认恢复
          </Button>
        </DialogActions>
      </Dialog>

      {/* 删除确认对话框 */}
      <Dialog open={deleteDialog} onClose={() => setDeleteDialog(false)}>
        <DialogTitle>确认删除备份</DialogTitle>
        <DialogContent>
          <Typography>
            确定要删除以下备份吗？此操作无法撤销！
          </Typography>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {selectedBackup}
              </Typography>
            </Box>
          )}
          <Alert severity="error" sx={{ mt: 2 }}>
            删除后将无法恢复此备份！
          </Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialog(false)}>
            取消
          </Button>
          <Button
            variant="contained"
            color="error"
            onClick={() => selectedBackup && handleDelete(selectedBackup)}
            disabled={loading}
          >
            确认删除
          </Button>
        </DialogActions>
      </Dialog>

      {/* 编辑标签对话框 */}
      <Dialog open={editDialog} onClose={() => setEditDialog(false)}>
        <DialogTitle>编辑备份标签</DialogTitle>
        <DialogContent>
          <Typography variant="body2" sx={{ mb: 2 }}>
            为备份添加自定义标签，方便识别：
          </Typography>
          {selectedBackup && (
            <Box sx={{ mb: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">
                {formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {selectedBackup}
              </Typography>
            </Box>
          )}
          <Box
            component="input"
            type="text"
            value={newLabel}
            onChange={(e: any) => setNewLabel(e.target.value)}
            placeholder="输入标签，例如：更新前备份"
            sx={{
              width: '100%',
              p: 1.5,
              border: '1px solid',
              borderColor: 'divider',
              borderRadius: 1,
              fontSize: '0.875rem',
              fontFamily: 'inherit',
              '&:focus': {
                outline: 'none',
                borderColor: 'primary.main',
              },
            }}
          />
          <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
            标签只能包含字母、数字、空格、下划线和连字符
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditDialog(false)}>
            取消
          </Button>
          <Button
            variant="contained"
            onClick={() => selectedBackup && handleRename(selectedBackup, newLabel)}
            disabled={loading || !newLabel.trim()}
          >
            保存
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
});

BackupManager.displayName = 'BackupManager';

export default BackupManager;
