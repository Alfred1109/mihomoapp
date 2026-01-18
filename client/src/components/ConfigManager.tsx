import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Settings, Save, RotateCcw, Upload, Download, FileText, Zap } from 'lucide-react';
import axios from 'axios';

interface ConfigManagerProps {}

const ConfigManager: React.FC<ConfigManagerProps> = () => {
  const [config, setConfig] = useState<any>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [configText, setConfigText] = useState('');
  const [activeTab, setActiveTab] = useState('general');

  // Load current configuration
  const loadConfig = async () => {
    setIsLoading(true);
    try {
      const response = await axios.get('/api/config');
      setConfig(response.data.config);
      setConfigText(JSON.stringify(response.data.config, null, 2));
      setHasChanges(false);
    } catch (error) {
      console.error('Failed to load configuration:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Save configuration
  const saveConfig = async () => {
    setIsLoading(true);
    try {
      await axios.post('/api/config', { config });
      setHasChanges(false);
      alert('Configuration saved successfully!');
    } catch (error) {
      console.error('Failed to save configuration:', error);
      alert('Failed to save configuration. Please check the logs.');
    } finally {
      setIsLoading(false);
    }
  };

  // Reset configuration to default
  const resetConfig = async () => {
    if (!confirm('Are you sure you want to reset the configuration to default? This will overwrite all current settings.')) {
      return;
    }

    setIsLoading(true);
    try {
      await axios.post('/api/config/reset');
      await loadConfig();
      alert('Configuration reset to default successfully!');
    } catch (error) {
      console.error('Failed to reset configuration:', error);
      alert('Failed to reset configuration. Please check the logs.');
    } finally {
      setIsLoading(false);
    }
  };

  // Update configuration field
  const updateConfig = (field: string, value: any) => {
    const newConfig = { ...config };
    
    // Handle nested fields (e.g., 'tun.enable')
    const fields = field.split('.');
    let current = newConfig;
    for (let i = 0; i < fields.length - 1; i++) {
      if (!current[fields[i]]) {
        current[fields[i]] = {};
      }
      current = current[fields[i]];
    }
    current[fields[fields.length - 1]] = value;
    
    setConfig(newConfig);
    setHasChanges(true);
  };

  // Save configuration from text editor
  const saveFromText = async () => {
    try {
      const parsedConfig = JSON.parse(configText);
      setConfig(parsedConfig);
      setHasChanges(true);
    } catch (error) {
      alert('Invalid JSON format. Please check your configuration.');
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  useEffect(() => {
    if (config) {
      setConfigText(JSON.stringify(config, null, 2));
    }
  }, [config]);

  if (!config) {
    return (
      <div className="space-y-6">
        <Card className="text-center py-12">
          <CardContent>
            <Settings className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">Loading Configuration</h3>
            <p className="text-muted-foreground">
              Please wait while we load the configuration...
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Configuration Manager</h2>
          <p className="text-muted-foreground">
            Manage your mihomo proxy configuration
          </p>
        </div>
        <div className="flex items-center space-x-2">
          {hasChanges && (
            <Badge variant="secondary">
              Unsaved Changes
            </Badge>
          )}
          <Button variant="outline" onClick={resetConfig} disabled={isLoading}>
            <RotateCcw className="mr-2 h-4 w-4" />
            Reset
          </Button>
          <Button onClick={saveConfig} disabled={isLoading || !hasChanges}>
            <Save className="mr-2 h-4 w-4" />
            Save
          </Button>
        </div>
      </div>

      {/* Configuration Tabs */}
      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid w-full grid-cols-4">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="proxy">Proxy</TabsTrigger>
          <TabsTrigger value="dns">DNS</TabsTrigger>
          <TabsTrigger value="advanced">Advanced</TabsTrigger>
        </TabsList>

        {/* General Configuration */}
        <TabsContent value="general" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Basic Settings</CardTitle>
              <CardDescription>
                Configure basic proxy and network settings
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-4 md:grid-cols-2">
                <div>
                  <Label htmlFor="port">HTTP Port</Label>
                  <Input
                    id="port"
                    type="number"
                    value={config.port || 7890}
                    onChange={(e) => updateConfig('port', parseInt(e.target.value))}
                  />
                </div>
                <div>
                  <Label htmlFor="socks-port">SOCKS Port</Label>
                  <Input
                    id="socks-port"
                    type="number"
                    value={config['socks-port'] || 7891}
                    onChange={(e) => updateConfig('socks-port', parseInt(e.target.value))}
                  />
                </div>
                <div>
                  <Label htmlFor="mixed-port">Mixed Port</Label>
                  <Input
                    id="mixed-port"
                    type="number"
                    value={config['mixed-port'] || 7890}
                    onChange={(e) => updateConfig('mixed-port', parseInt(e.target.value))}
                  />
                </div>
                <div>
                  <Label htmlFor="external-controller">External Controller</Label>
                  <Input
                    id="external-controller"
                    value={config['external-controller'] || '127.0.0.1:9090'}
                    onChange={(e) => updateConfig('external-controller', e.target.value)}
                  />
                </div>
              </div>

              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <Label htmlFor="allow-lan">Allow LAN Access</Label>
                    <p className="text-sm text-muted-foreground">
                      Allow other devices on your network to use this proxy
                    </p>
                  </div>
                  <Switch
                    id="allow-lan"
                    checked={config['allow-lan'] || false}
                    onCheckedChange={(checked) => updateConfig('allow-lan', checked)}
                  />
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <Label htmlFor="ipv6">IPv6 Support</Label>
                    <p className="text-sm text-muted-foreground">
                      Enable IPv6 traffic handling
                    </p>
                  </div>
                  <Switch
                    id="ipv6"
                    checked={config.ipv6 !== false}
                    onCheckedChange={(checked) => updateConfig('ipv6', checked)}
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          {/* TUN Mode Configuration */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center space-x-2">
                <Zap className="h-5 w-5" />
                <span>TUN Mode</span>
              </CardTitle>
              <CardDescription>
                System-wide transparent proxy configuration
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="tun-enable">Enable TUN Mode</Label>
                  <p className="text-sm text-muted-foreground">
                    Requires administrator privileges
                  </p>
                </div>
                <Switch
                  id="tun-enable"
                  checked={config.tun?.enable || false}
                  onCheckedChange={(checked) => updateConfig('tun.enable', checked)}
                />
              </div>

              {config.tun?.enable && (
                <div className="grid gap-4 md:grid-cols-2">
                  <div>
                    <Label htmlFor="tun-stack">TUN Stack</Label>
                    <select
                      id="tun-stack"
                      className="w-full h-10 px-3 py-2 text-sm border border-input rounded-md bg-background"
                      value={config.tun?.stack || 'system'}
                      onChange={(e) => updateConfig('tun.stack', e.target.value)}
                    >
                      <option value="system">System</option>
                      <option value="gvisor">gVisor</option>
                      <option value="lwip">LWIP</option>
                    </select>
                  </div>
                  <div>
                    <Label htmlFor="tun-device">TUN Device Name</Label>
                    <Input
                      id="tun-device"
                      value={config.tun?.['device-name'] || ''}
                      onChange={(e) => updateConfig('tun.device-name', e.target.value)}
                      placeholder="auto"
                    />
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Proxy Configuration */}
        <TabsContent value="proxy" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Proxy Settings</CardTitle>
              <CardDescription>
                Configure proxy mode and routing behavior
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <Label htmlFor="mode">Proxy Mode</Label>
                <select
                  id="mode"
                  className="w-full h-10 px-3 py-2 text-sm border border-input rounded-md bg-background"
                  value={config.mode || 'rule'}
                  onChange={(e) => updateConfig('mode', e.target.value)}
                >
                  <option value="rule">Rule-based</option>
                  <option value="global">Global</option>
                  <option value="direct">Direct</option>
                </select>
              </div>

              <div>
                <Label htmlFor="log-level">Log Level</Label>
                <select
                  id="log-level"
                  className="w-full h-10 px-3 py-2 text-sm border border-input rounded-md bg-background"
                  value={config['log-level'] || 'info'}
                  onChange={(e) => updateConfig('log-level', e.target.value)}
                >
                  <option value="silent">Silent</option>
                  <option value="error">Error</option>
                  <option value="warning">Warning</option>
                  <option value="info">Info</option>
                  <option value="debug">Debug</option>
                </select>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* DNS Configuration */}
        <TabsContent value="dns" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>DNS Settings</CardTitle>
              <CardDescription>
                Configure DNS resolution and enhancement
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="dns-enable">Enable DNS</Label>
                  <p className="text-sm text-muted-foreground">
                    Use mihomo's built-in DNS server
                  </p>
                </div>
                <Switch
                  id="dns-enable"
                  checked={config.dns?.enable !== false}
                  onCheckedChange={(checked) => updateConfig('dns.enable', checked)}
                />
              </div>

              {config.dns?.enable !== false && (
                <div className="space-y-4">
                  <div>
                    <Label htmlFor="dns-listen">DNS Listen Address</Label>
                    <Input
                      id="dns-listen"
                      value={config.dns?.listen || '0.0.0.0:53'}
                      onChange={(e) => updateConfig('dns.listen', e.target.value)}
                    />
                  </div>

                  <div>
                    <Label htmlFor="dns-enhanced-mode">Enhanced Mode</Label>
                    <select
                      id="dns-enhanced-mode"
                      className="w-full h-10 px-3 py-2 text-sm border border-input rounded-md bg-background"
                      value={config.dns?.enhanced_mode || 'fake-ip'}
                      onChange={(e) => updateConfig('dns.enhanced_mode', e.target.value)}
                    >
                      <option value="fake-ip">Fake IP</option>
                      <option value="redir-host">Redirect Host</option>
                    </select>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Advanced Configuration */}
        <TabsContent value="advanced" className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center space-x-2">
                <FileText className="h-5 w-5" />
                <span>Raw Configuration</span>
              </CardTitle>
              <CardDescription>
                Edit the raw configuration file directly (JSON format)
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Textarea
                value={configText}
                onChange={(e) => setConfigText(e.target.value)}
                className="min-h-96 font-mono text-sm"
                placeholder="Configuration JSON..."
              />
              <div className="flex justify-end space-x-2">
                <Button variant="outline" onClick={loadConfig}>
                  <Download className="mr-2 h-4 w-4" />
                  Reload
                </Button>
                <Button onClick={saveFromText}>
                  <Upload className="mr-2 h-4 w-4" />
                  Apply Changes
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default ConfigManager;
