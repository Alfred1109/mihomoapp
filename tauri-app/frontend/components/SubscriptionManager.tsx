import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Button,
  Card,
  CardContent,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  TextField,
  Typography,
  Chip,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  LinearProgress,
  FormControlLabel,
  Switch,
  Divider,
  List,
  ListItem,
  ListItemText,
  ListItemSecondaryAction,
  Alert,
} from '@mui/material';
import {
  Add,
  Delete,
  Refresh,
  Link,
  CheckCircle,
  Error,
  Update,
  Download,
  Upload,
  History,
  Restore,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import { useTranslation } from 'react-i18next';

import type { Subscription } from '../types';

interface SubscriptionManagerProps {
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const SubscriptionManager: React.FC<SubscriptionManagerProps> = React.memo(({ showNotification }) => {
  const { t } = useTranslation();
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [loading, setLoading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<{ id: string; name: string } | null>(null);
  const [newSubscription, setNewSubscription] = useState({
    name: '',
    url: '',
    userAgent: 'clash',
    useProxy: false,
  });
  
  const [subscriptionBackups, setSubscriptionBackups] = useState<string[]>([]);
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false);
  const [selectedBackup, setSelectedBackup] = useState<string | null>(null);

  const loadSubscriptions = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<Subscription[]>('get_subscriptions');
      setSubscriptions(data);
    } catch (error) {
      showNotification(`${t('subscription.loadFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showNotification, t]);

  const loadBackups = useCallback(async () => {
    try {
      const result = await invoke<string[]>('list_subscription_backups');
      setSubscriptionBackups(result);
    } catch {
    }
  }, []);

  const handleAddSubscription = async () => {
    if (!newSubscription.name || !newSubscription.url) {
      showNotification(t('subscription.fillRequired'), 'error');
      return;
    }
    try {
      await invoke('add_subscription', {
        name: newSubscription.name,
        url: newSubscription.url,
        userAgent: newSubscription.userAgent || null,
        useProxy: newSubscription.useProxy,
      });
      showNotification(t('subscription.addSuccess'), 'success');
      setDialogOpen(false);
      setNewSubscription({ name: '', url: '', userAgent: 'clash', useProxy: false });
      loadSubscriptions();
    } catch (error) {
      showNotification(`${t('subscription.addFailed')}: ${error}`, 'error');
    }
  };

  const handleUpdateSubscription = async (id: string) => {
    try {
      await invoke('update_subscription', { id });
      showNotification(t('subscription.updateSuccess'), 'success');
      loadSubscriptions();
    } catch (error) {
      showNotification(`${t('subscription.updateFailed')}: ${error}`, 'error');
    }
  };

  const handleDeleteSubscription = async () => {
    if (!deleteTarget) return;
    try {
      await invoke('delete_subscription', { id: deleteTarget.id });
      showNotification(t('subscription.deleteSuccess'), 'success');
      setDeleteDialogOpen(false);
      setDeleteTarget(null);
      loadSubscriptions();
    } catch (error) {
      showNotification(`${t('subscription.deleteFailed')}: ${error}`, 'error');
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
        showNotification(t('subscription.exportSuccess'), 'success');
      }
    } catch (error) {
      showNotification(`${t('subscription.exportFailed')}: ${error}`, 'error');
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
        const count = await invoke<number>('import_subscriptions', { json_content: content });
        showNotification(t('subscription.importSuccess', { count }), 'success');
        loadSubscriptions();
      }
    } catch (error) {
      showNotification(`${t('subscription.importFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleBackupSubscriptions = async () => {
    setLoading(true);
    try {
      const filename = await invoke<string>('backup_subscriptions');
      showNotification(`${t('subscription.backupSuccess')}: ${filename}`, 'success');
      loadBackups();
    } catch (error) {
      showNotification(`${t('subscription.backupFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreSubscriptions = async (backupFilename: string) => {
    setLoading(true);
    try {
      const count = await invoke<number>('restore_subscriptions_from_backup', { backup_filename: backupFilename });
      showNotification(t('subscription.restoreSuccess', { count }), 'success');
      setRestoreDialogOpen(false);
      setSelectedBackup(null);
      loadSubscriptions();
    } catch (error) {
      showNotification(`${t('subscription.restoreFailed')}: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString();
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

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Active': return <CheckCircle color="success" />;
      case 'Error': return <Error color="error" />;
      case 'Updating': return <Update color="info" />;
      default: return <CheckCircle color="success" />;
    }
  };

  const getStatusColor = (status: string): 'success' | 'error' | 'info' | 'default' => {
    switch (status) {
      case 'Active': return 'success';
      case 'Error': return 'error';
      case 'Updating': return 'info';
      default: return 'default';
    }
  };

  useEffect(() => {
    loadSubscriptions();
    loadBackups();
  }, [loadSubscriptions, loadBackups]);

  return (
    <Box>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">{t('subscription.title')}</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button variant="outlined" startIcon={<Refresh />} onClick={loadSubscriptions} disabled={loading}>
            {t('subscription.refresh')}
          </Button>
          <Button variant="contained" startIcon={<Add />} onClick={() => setDialogOpen(true)}>
            {t('subscription.add')}
          </Button>
        </Box>
      </Box>

      {loading && <LinearProgress sx={{ mb: 2 }} />}

      <Card sx={{ mb: 3 }}>
        <CardContent>
          {subscriptions.length === 0 ? (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Link sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
              <Typography variant="h6" gutterBottom>{t('subscription.noSubscriptions')}</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>{t('subscription.noSubscriptionsDesc')}</Typography>
              <Button variant="contained" startIcon={<Add />} onClick={() => setDialogOpen(true)}>
                {t('subscription.add')}
              </Button>
            </Box>
          ) : (
            <TableContainer component={Paper}>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>{t('subscription.name')}</TableCell>
                    <TableCell>{t('subscription.url')}</TableCell>
                    <TableCell>{t('subscription.status')}</TableCell>
                    <TableCell>{t('subscription.proxies')}</TableCell>
                    <TableCell>{t('subscription.lastUpdated')}</TableCell>
                    <TableCell>{t('subscription.actions')}</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {subscriptions.map((subscription) => (
                    <TableRow key={subscription.id}>
                      <TableCell>
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          {getStatusIcon(subscription.status)}
                          <Typography variant="subtitle2">{subscription.name}</Typography>
                        </Box>
                      </TableCell>
                      <TableCell>
                        <Typography variant="body2" sx={{ maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                          {subscription.url}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
                          <Chip label={subscription.status} color={getStatusColor(subscription.status)} size="small" />
                          <Chip label={subscription.use_proxy ? t('subscription.useProxyLabel') : t('subscription.directLabel')} size="small" variant="outlined"
                            color={subscription.use_proxy ? 'primary' : 'default'} />
                          {subscription.last_error && (
                            <Typography variant="caption" color="error" sx={{ display: 'block', mt: 0.5 }}>
                              {subscription.last_error}
                            </Typography>
                          )}
                        </Box>
                      </TableCell>
                      <TableCell>{subscription.proxy_count}</TableCell>
                      <TableCell>{formatDate(subscription.last_updated)}</TableCell>
                      <TableCell>
                        <Box sx={{ display: 'flex', gap: 1 }}>
                          <IconButton size="small" onClick={() => handleUpdateSubscription(subscription.id)} title={t('subscription.update')}>
                            <Refresh />
                          </IconButton>
                          <IconButton size="small" color="error" title={t('subscription.delete')}
                            onClick={() => { setDeleteTarget({ id: subscription.id, name: subscription.name }); setDeleteDialogOpen(true); }}>
                            <Delete />
                          </IconButton>
                        </Box>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </TableContainer>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardContent>
          <Typography variant="h6" gutterBottom>{t('subscription.dataManagement')}</Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            {t('subscription.dataManagementDesc')}
          </Typography>
          <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', mb: 3 }}>
            <Button variant="outlined" startIcon={<Download />} onClick={handleExportSubscriptions} disabled={loading}>
              {t('subscription.export')}
            </Button>
            <Button variant="outlined" startIcon={<Upload />} onClick={handleImportSubscriptions} disabled={loading}>
              {t('subscription.import')}
            </Button>
            <Button variant="contained" startIcon={<History />} onClick={handleBackupSubscriptions} disabled={loading}>
              {t('subscription.createBackup')}
            </Button>
          </Box>

          {subscriptionBackups.length > 0 && (
            <>
              <Divider sx={{ my: 2 }} />
              <Typography variant="subtitle1" sx={{ mb: 2 }}>{t('subscription.backups')} ({subscriptionBackups.length})</Typography>
              <List>
                {subscriptionBackups.slice(0, 5).map((backup, index) => {
                  const { date, time } = formatBackupName(backup);
                  const isLatest = index === 0;
                  return (
                    <ListItem key={backup} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 1, mb: 1,
                      bgcolor: isLatest ? 'action.hover' : 'background.paper' }}>
                      <ListItemText
                        primary={
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                            <Typography variant="subtitle2">{date} {time}</Typography>
                            {isLatest && <Chip icon={<CheckCircle />} label={t('subscription.latest')} size="small" color="success" />}
                          </Box>
                        }
                      />
                      <ListItemSecondaryAction>
                        <IconButton size="small" color="primary" title={t('subscription.restoreThis')}
                          onClick={() => { setSelectedBackup(backup); setRestoreDialogOpen(true); }}>
                          <Restore fontSize="small" />
                        </IconButton>
                      </ListItemSecondaryAction>
                    </ListItem>
                  );
                })}
              </List>
            </>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>{t('subscription.addDialogTitle')}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>{t('subscription.addDialogDesc')}</DialogContentText>
          <TextField autoFocus margin="dense" label={t('subscription.subscriptionName')} fullWidth variant="outlined"
            value={newSubscription.name} onChange={(e) => setNewSubscription({ ...newSubscription, name: e.target.value })} sx={{ mb: 2 }} />
          <TextField margin="dense" label={t('subscription.subscriptionUrl')} fullWidth variant="outlined"
            value={newSubscription.url} onChange={(e) => setNewSubscription({ ...newSubscription, url: e.target.value })} sx={{ mb: 2 }} />
          <TextField margin="dense" label={t('subscription.userAgent')} fullWidth variant="outlined"
            value={newSubscription.userAgent} onChange={(e) => setNewSubscription({ ...newSubscription, userAgent: e.target.value })} sx={{ mb: 2 }} />
          <FormControlLabel control={<Switch checked={newSubscription.useProxy}
            onChange={(e) => setNewSubscription({ ...newSubscription, useProxy: e.target.checked })} color="primary" />}
            label={t('subscription.useProxy')} />
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>{t('subscription.useProxyDesc')}</Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)}>{t('subscription.cancel')}</Button>
          <Button onClick={handleAddSubscription} variant="contained">{t('subscription.add')}</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={deleteDialogOpen} onClose={() => setDeleteDialogOpen(false)}>
        <DialogTitle>{t('subscription.confirmDelete')}</DialogTitle>
        <DialogContent>
          <DialogContentText>{t('subscription.confirmDeleteDesc', { name: deleteTarget?.name })}</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDeleteDialogOpen(false)}>{t('common.cancel')}</Button>
          <Button variant="contained" color="error" onClick={handleDeleteSubscription}>{t('common.confirm')}</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={restoreDialogOpen} onClose={() => setRestoreDialogOpen(false)}>
        <DialogTitle>{t('subscription.confirmRestore')}</DialogTitle>
        <DialogContent>
          <DialogContentText>{t('subscription.confirmRestoreDesc')}</DialogContentText>
          {selectedBackup && (
            <Box sx={{ mt: 2, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
              <Typography variant="body2">{formatBackupName(selectedBackup).date} {formatBackupName(selectedBackup).time}</Typography>
            </Box>
          )}
          <Alert severity="info" sx={{ mt: 2 }}>{t('subscription.restoreNote')}</Alert>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setRestoreDialogOpen(false)}>{t('common.cancel')}</Button>
          <Button variant="contained" onClick={() => selectedBackup && handleRestoreSubscriptions(selectedBackup)}>{t('common.confirm')}</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
});

SubscriptionManager.displayName = 'SubscriptionManager';

export default SubscriptionManager;
