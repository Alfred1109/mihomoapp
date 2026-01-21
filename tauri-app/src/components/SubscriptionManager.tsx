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
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [loading, setLoading] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [newSubscription, setNewSubscription] = useState({
    name: '',
    url: '',
    userAgent: 'clash',
    useProxy: false,
  });
  const [selectedSubscriptions, setSelectedSubscriptions] = useState<string[]>([]);

  const loadSubscriptions = async () => {
    setLoading(true);
    try {
      const data = await invoke<Subscription[]>('get_subscriptions');
      setSubscriptions(data);
    } catch (error) {
      showNotification(`Failed to load subscriptions: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleAddSubscription = async () => {
    if (!newSubscription.name || !newSubscription.url) {
      showNotification('Please fill in all required fields', 'error');
      return;
    }

    try {
      await invoke('add_subscription', {
        name: newSubscription.name,
        url: newSubscription.url,
        userAgent: newSubscription.userAgent || null,
        useProxy: newSubscription.useProxy,
      });
      
      showNotification('Subscription added successfully', 'success');
      setDialogOpen(false);
      setNewSubscription({ name: '', url: '', userAgent: 'clash', useProxy: false });
      loadSubscriptions();
    } catch (error) {
      showNotification(`Failed to add subscription: ${error}`, 'error');
    }
  };

  const handleUpdateSubscription = async (id: string) => {
    try {
      await invoke('update_subscription', { id });
      showNotification('Subscription updated successfully', 'success');
      loadSubscriptions();
    } catch (error) {
      showNotification(`Failed to update subscription: ${error}`, 'error');
    }
  };

  const handleDeleteSubscription = async (id: string, name: string) => {
    if (!confirm(`Are you sure you want to delete subscription "${name}"?`)) {
      return;
    }

    try {
      await invoke('delete_subscription', { id });
      showNotification('Subscription deleted successfully', 'success');
      loadSubscriptions();
    } catch (error) {
      showNotification(`Failed to delete subscription: ${error}`, 'error');
    }
  };

  const handleGenerateConfig = async () => {
    if (selectedSubscriptions.length === 0) {
      showNotification('Please select at least one subscription', 'error');
      return;
    }

    try {
      await invoke('generate_config_from_subscriptions', {
        subscriptionIds: selectedSubscriptions,
      });
      showNotification('Configuration generated successfully', 'success');
    } catch (error) {
      showNotification(`Failed to generate configuration: ${error}`, 'error');
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
        <Typography variant="h5">Subscription Manager</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            variant="outlined"
            startIcon={<Refresh />}
            onClick={loadSubscriptions}
            disabled={loading}
          >
            Refresh
          </Button>
          <Button
            variant="contained"
            startIcon={<Add />}
            onClick={() => setDialogOpen(true)}
          >
            Add Subscription
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
                No Subscriptions
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Add your first subscription to get started
              </Typography>
              <Button variant="contained" startIcon={<Add />} onClick={() => setDialogOpen(true)}>
                Add Subscription
              </Button>
            </Box>
          ) : (
            <TableContainer component={Paper}>
              <Table>
                <TableHead>
                  <TableRow>
                    <TableCell>Name</TableCell>
                    <TableCell>URL</TableCell>
                    <TableCell>Status</TableCell>
                    <TableCell>Proxies</TableCell>
                    <TableCell>Last Updated</TableCell>
                    <TableCell>Actions</TableCell>
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
                            title="Update"
                          >
                            <Refresh />
                          </IconButton>
                          <IconButton
                            size="small"
                            onClick={() => handleDeleteSubscription(subscription.id, subscription.name)}
                            title="Delete"
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

      {/* Quick Actions */}
      {subscriptions.length > 0 && (
        <Card>
          <CardContent>
            <Typography variant="h6" gutterBottom>
              Quick Actions
            </Typography>
            <Button
              variant="contained"
              startIcon={<CloudDownload />}
              onClick={handleGenerateConfig}
              disabled={subscriptions.filter(s => s.status === 'Active').length === 0}
              fullWidth
            >
              Generate Config from All Active Subscriptions
            </Button>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 2, textAlign: 'center' }}>
              This will merge all active subscriptions and update your mihomo configuration
            </Typography>
          </CardContent>
        </Card>
      )}

      {/* Add Subscription Dialog */}
      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Add New Subscription</DialogTitle>
        <DialogContent>
          <DialogContentText sx={{ mb: 2 }}>
            Add a new proxy subscription URL to import configurations
          </DialogContentText>
          <TextField
            autoFocus
            margin="dense"
            label="Subscription Name"
            fullWidth
            variant="outlined"
            value={newSubscription.name}
            onChange={(e) => setNewSubscription({ ...newSubscription, name: e.target.value })}
            sx={{ mb: 2 }}
          />
          <TextField
            margin="dense"
            label="Subscription URL"
            fullWidth
            variant="outlined"
            value={newSubscription.url}
            onChange={(e) => setNewSubscription({ ...newSubscription, url: e.target.value })}
            sx={{ mb: 2 }}
          />
          <TextField
            margin="dense"
            label="User Agent"
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
            label="使用代理下载订阅"
          />
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
            如果订阅链接需要直连访问（不通过代理），请关闭此开关
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleAddSubscription} variant="contained">
            Add
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default SubscriptionManager;
