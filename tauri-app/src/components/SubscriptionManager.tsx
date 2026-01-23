import React, { useState, useEffect } from 'react';
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
  Grid,
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
} from '@mui/material';
import {
  Add,
  Delete,
  Refresh,
  CloudDownload,
  Link,
  CheckCircle,
  Error,
  Update,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';

interface SubscriptionManagerProps {
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

interface Subscription {
  id: string;
  name: string;
  url: string;
  user_agent?: string;
  use_proxy: boolean;
  created_at: string;
  last_updated: string;
  proxy_count: number;
  status: 'Active' | 'Error' | 'Updating';
  last_error?: string;
}

const SubscriptionManager: React.FC<SubscriptionManagerProps> = ({ showNotification }) => {
  const { t } = useTranslation();
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [loading, setLoading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newSubscription, setNewSubscription] = useState({
    name: '',
    url: '',
    userAgent: 'clash',
    useProxy: false,
  });

  const loadSubscriptions = async () => {
    setLoading(true);
    try {
      const data = await invoke<Subscription[]>('get_subscriptions');
      setSubscriptions(data);
    } catch (error) {
      showNotification(`加载订阅失败: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleAddSubscription = async () => {
    if (!newSubscription.name || !newSubscription.url) {
      showNotification('请填写所有必填字段', 'error');
      return;
    }

    try {
      await invoke('add_subscription', {
        name: newSubscription.name,
        url: newSubscription.url,
        userAgent: newSubscription.userAgent || null,
        useProxy: newSubscription.useProxy,
      });
      
      showNotification('订阅添加成功', 'success');
      setDialogOpen(false);
      setNewSubscription({ name: '', url: '', userAgent: 'clash', useProxy: false });
      loadSubscriptions();
    } catch (error) {
      showNotification(`添加订阅失败: ${error}`, 'error');
    }
  };

  const handleUpdateSubscription = async (id: string) => {
    try {
      await invoke('update_subscription', { id });
      showNotification('订阅更新成功！配置文件已生成。如果服务正在运行，请重启以应用更改。', 'success');
      loadSubscriptions();
    } catch (error) {
      showNotification(`更新订阅失败: ${error}`, 'error');
    }
  };

  const handleDeleteSubscription = async (id: string, name: string) => {
    if (!confirm(`确定要删除订阅 "${name}" 吗？`)) {
      return;
    }

    try {
      await invoke('delete_subscription', { id });
      showNotification('订阅删除成功', 'success');
      loadSubscriptions();
    } catch (error) {
      showNotification(`删除订阅失败: ${error}`, 'error');
    }
  };

  const handleGenerateConfig = async () => {
    // 自动使用所有活跃的订阅
    const activeSubscriptions = subscriptions
      .filter(s => s.status === 'Active')
      .map(s => s.id);
    
    if (activeSubscriptions.length === 0) {
      showNotification('没有活跃的订阅，请先添加并更新订阅', 'error');
      return;
    }

    try {
      await invoke('generate_config_from_subscriptions', {
        subscriptionIds: activeSubscriptions,
      });
      showNotification(`配置生成成功！已合并 ${activeSubscriptions.length} 个订阅`, 'success');
    } catch (error) {
      showNotification(`生成配置失败: ${error}`, 'error');
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString();
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Active':
        return <CheckCircle color="success" />;
      case 'Error':
        return <Error color="error" />;
      case 'Updating':
        return <Update color="info" />;
      default:
        return <CheckCircle color="success" />;
    }
  };

  const getStatusColor = (status: string): 'success' | 'error' | 'info' | 'default' => {
    switch (status) {
      case 'Active':
        return 'success';
      case 'Error':
        return 'error';
      case 'Updating':
        return 'info';
      default:
        return 'default';
    }
  };

  useEffect(() => {
    loadSubscriptions();
  }, []);

  return (
    <Box>
      {/* Header */}
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5">{t('subscription.title')}</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<Refresh />}
            onClick={loadSubscriptions}
            disabled={loading}
          >
            {t('subscription.refresh')}
          </Button>
          <Button
            variant="contained"
            startIcon={<Add />}
            onClick={() => setDialogOpen(true)}
          >
            {t('subscription.add')}
          </Button>
        </Box>
      </Box>

      {loading && <LinearProgress sx={{ mb: 2 }} />}

      {/* Subscriptions Table */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          {subscriptions.length === 0 ? (
            <Box sx={{ textAlign: 'center', py: 4 }}>
              <Link sx={{ fontSize: 48, color: 'text.secondary', mb: 2 }} />
              <Typography variant="h6" gutterBottom>
                {t('subscription.noSubscriptions')}
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                {t('subscription.noSubscriptionsDesc')}
              </Typography>
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
                        <Typography
                          variant="body2"
                          sx={{
                            maxWidth: 300,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {subscription.url}
                        </Typography>
                      </TableCell>
                      <TableCell>
                        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
                          <Chip
                            label={subscription.status}
                            color={getStatusColor(subscription.status)}
                            size="small"
                          />
                          <Chip
                            label={subscription.use_proxy ? '使用代理' : '直连'}
                            size="small"
                            variant="outlined"
                            color={subscription.use_proxy ? 'primary' : 'default'}
                          />
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
                          <IconButton
                            size="small"
                            onClick={() => handleUpdateSubscription(subscription.id)}
                            title={t('subscription.update')}
                          >
                            <Refresh />
                          </IconButton>
                          <IconButton
                            size="small"
                            onClick={() => handleDeleteSubscription(subscription.id, subscription.name)}
                            title={t('subscription.delete')}
                            color="error"
                          >
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

      {/* Add Subscription Dialog */}
      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>{t('subscription.addDialogTitle')}</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>
            {t('subscription.addDialogDesc')}
          </DialogContentText>
          <TextField
            autoFocus
            margin="dense"
            label={t('subscription.subscriptionName')}
            fullWidth
            variant="outlined"
            value={newSubscription.name}
            onChange={(e) => setNewSubscription({ ...newSubscription, name: e.target.value })}
            sx={{ mb: 2 }}
          />
          <TextField
            margin="dense"
            label={t('subscription.subscriptionUrl')}
            fullWidth
            variant="outlined"
            value={newSubscription.url}
            onChange={(e) => setNewSubscription({ ...newSubscription, url: e.target.value })}
            sx={{ mb: 2 }}
          />
          <TextField
            margin="dense"
            label={t('subscription.userAgent')}
            fullWidth
            variant="outlined"
            value={newSubscription.userAgent}
            onChange={(e) => setNewSubscription({ ...newSubscription, userAgent: e.target.value })}
            sx={{ mb: 2 }}
          />
          <FormControlLabel
            control={
              <Switch
                checked={newSubscription.useProxy}
                onChange={(e) => setNewSubscription({ ...newSubscription, useProxy: e.target.checked })}
                color="primary"
              />
            }
            label={t('subscription.useProxy')}
          />
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
            {t('subscription.useProxyDesc')}
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)}>{t('subscription.cancel')}</Button>
          <Button onClick={handleAddSubscription} variant="contained">
            {t('subscription.add')}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default SubscriptionManager;
