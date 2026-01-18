import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
import { AlertCircle, Plus, RefreshCw, Trash2, Download, Globe, Clock, CheckCircle } from 'lucide-react';
import axios from 'axios';

interface Subscription {
  id: string;
  name: string;
  url: string;
  userAgent?: string;
  createdAt: string;
  lastUpdated: string;
  proxyCount: number;
  status: 'active' | 'error';
  lastError?: string;
}

const SubscriptionManager: React.FC = () => {
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [newSubscription, setNewSubscription] = useState({
    name: '',
    url: '',
    userAgent: 'clash'
  });

  // Load subscriptions
  const loadSubscriptions = async () => {
    try {
      const response = await axios.get('/api/subscription');
      setSubscriptions(response.data);
    } catch (error) {
      console.error('Failed to load subscriptions:', error);
    }
  };

  // Add new subscription
  const addSubscription = async () => {
    if (!newSubscription.name || !newSubscription.url) {
      return;
    }

    setIsLoading(true);
    try {
      await axios.post('/api/subscription', newSubscription);
      setShowAddDialog(false);
      setNewSubscription({ name: '', url: '', userAgent: 'clash' });
      await loadSubscriptions();
    } catch (error) {
      console.error('Failed to add subscription:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Refresh subscription
  const refreshSubscription = async (id: string) => {
    setIsLoading(true);
    try {
      await axios.post(`/api/subscription/${id}/refresh`);
      await loadSubscriptions();
    } catch (error) {
      console.error('Failed to refresh subscription:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Delete subscription
  const deleteSubscription = async (id: string) => {
    if (!confirm('Are you sure you want to delete this subscription?')) {
      return;
    }

    setIsLoading(true);
    try {
      await axios.delete(`/api/subscription/${id}`);
      await loadSubscriptions();
    } catch (error) {
      console.error('Failed to delete subscription:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Generate merged config from selected subscriptions
  const generateConfig = async (subscriptionIds: string[]) => {
    if (subscriptionIds.length === 0) {
      alert('Please select at least one subscription');
      return;
    }

    setIsLoading(true);
    try {
      const response = await axios.post('/api/subscription/merge', {
        subscriptionIds,
        mergeOptions: {
          port: 7890,
          'socks-port': 7891,
          'allow-lan': false,
          mode: 'rule',
          'log-level': 'info'
        }
      });

      // Save merged config
      await axios.post('/api/config', { config: response.data.config });
      alert(`Configuration generated successfully with ${response.data.stats.totalProxies} proxies`);
    } catch (error) {
      console.error('Failed to generate config:', error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadSubscriptions();
  }, []);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Subscription Manager</h2>
          <p className="text-muted-foreground">
            Manage your proxy subscriptions and configurations
          </p>
        </div>
        <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
          <DialogTrigger asChild>
            <Button>
              <Plus className="mr-2 h-4 w-4" />
              Add Subscription
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Add New Subscription</DialogTitle>
              <DialogDescription>
                Add a new proxy subscription URL to import configurations
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4">
              <div>
                <Label htmlFor="name">Name</Label>
                <Input
                  id="name"
                  placeholder="My Subscription"
                  value={newSubscription.name}
                  onChange={(e) => setNewSubscription({ ...newSubscription, name: e.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="url">Subscription URL</Label>
                <Input
                  id="url"
                  placeholder="https://example.com/subscription"
                  value={newSubscription.url}
                  onChange={(e) => setNewSubscription({ ...newSubscription, url: e.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="userAgent">User Agent</Label>
                <Input
                  id="userAgent"
                  placeholder="clash"
                  value={newSubscription.userAgent}
                  onChange={(e) => setNewSubscription({ ...newSubscription, userAgent: e.target.value })}
                />
              </div>
              <div className="flex justify-end space-x-2">
                <Button variant="outline" onClick={() => setShowAddDialog(false)}>
                  Cancel
                </Button>
                <Button onClick={addSubscription} disabled={isLoading}>
                  {isLoading ? <RefreshCw className="mr-2 h-4 w-4 animate-spin" /> : <Plus className="mr-2 h-4 w-4" />}
                  Add
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </div>

      {/* Subscriptions List */}
      <div className="grid gap-4">
        {subscriptions.length === 0 ? (
          <Card className="text-center py-12">
            <CardContent>
              <Globe className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-medium mb-2">No Subscriptions</h3>
              <p className="text-muted-foreground mb-4">
                Get started by adding your first proxy subscription
              </p>
              <Button onClick={() => setShowAddDialog(true)}>
                <Plus className="mr-2 h-4 w-4" />
                Add Subscription
              </Button>
            </CardContent>
          </Card>
        ) : (
          subscriptions.map((subscription) => (
            <Card key={subscription.id} className="relative">
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <CardTitle className="flex items-center space-x-2">
                      <span>{subscription.name}</span>
                      <Badge 
                        variant={subscription.status === 'active' ? 'default' : 'destructive'}
                        className="text-xs"
                      >
                        {subscription.status === 'active' ? (
                          <><CheckCircle className="mr-1 h-3 w-3" />Active</>
                        ) : (
                          <><AlertCircle className="mr-1 h-3 w-3" />Error</>
                        )}
                      </Badge>
                    </CardTitle>
                    <CardDescription className="break-all">
                      {subscription.url}
                    </CardDescription>
                  </div>
                  <div className="flex items-center space-x-1">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => refreshSubscription(subscription.id)}
                      disabled={isLoading}
                    >
                      <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => deleteSubscription(subscription.id)}
                      disabled={isLoading}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="grid gap-4 md:grid-cols-3">
                  <div>
                    <p className="text-sm font-medium">Proxy Count</p>
                    <p className="text-2xl font-bold">{subscription.proxyCount}</p>
                  </div>
                  <div>
                    <p className="text-sm font-medium">Last Updated</p>
                    <div className="flex items-center text-sm text-muted-foreground">
                      <Clock className="mr-1 h-3 w-3" />
                      {new Date(subscription.lastUpdated).toLocaleDateString()}
                    </div>
                  </div>
                  <div>
                    <p className="text-sm font-medium">User Agent</p>
                    <p className="text-sm text-muted-foreground">{subscription.userAgent || 'clash'}</p>
                  </div>
                </div>

                {subscription.status === 'error' && subscription.lastError && (
                  <div className="mt-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md">
                    <div className="flex items-center space-x-2">
                      <AlertCircle className="h-4 w-4 text-destructive" />
                      <p className="text-sm font-medium text-destructive">Error</p>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1 break-all">
                      {subscription.lastError}
                    </p>
                  </div>
                )}
              </CardContent>
            </Card>
          ))
        )}
      </div>

      {/* Quick Actions */}
      {subscriptions.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Quick Actions</CardTitle>
            <CardDescription>
              Generate configuration from your subscriptions
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <Button
                onClick={() => generateConfig(subscriptions.filter(s => s.status === 'active').map(s => s.id))}
                disabled={isLoading || subscriptions.filter(s => s.status === 'active').length === 0}
                className="w-full"
              >
                <Download className="mr-2 h-4 w-4" />
                Generate Config from All Active Subscriptions
              </Button>
              
              <div className="text-center">
                <p className="text-sm text-muted-foreground">
                  This will merge all active subscriptions and update your mihomo configuration
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

export default SubscriptionManager;
