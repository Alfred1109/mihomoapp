import React, { useState, useEffect } from 'react';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { AlertCircle, Activity, Settings, Globe, Zap, Play, Square } from 'lucide-react';

import Dashboard from '@/components/Dashboard';
import SubscriptionManager from '@/components/SubscriptionManager';
import ProxyManager from '@/components/ProxyManager';
import ConfigManager from '@/components/ConfigManager';
import { useWebSocket } from '@/hooks/useWebSocket';
import { useMihomoAPI } from '@/hooks/useMihomoAPI';

function App() {
  const [activeTab, setActiveTab] = useState('dashboard');
  const [mihomoStatus, setMihomoStatus] = useState({
    isRunning: false,
    processId: null,
    apiStatus: null
  });

  const { messages } = useWebSocket();
  const { status, config, startMihomo, stopMihomo, toggleTun } = useMihomoAPI();

  useEffect(() => {
    if (status) {
      setMihomoStatus(status);
    }
  }, [status]);

  const handleStart = async () => {
    try {
      await startMihomo();
    } catch (error) {
      console.error('Failed to start mihomo:', error);
    }
  };

  const handleStop = async () => {
    try {
      await stopMihomo();
    } catch (error) {
      console.error('Failed to stop mihomo:', error);
    }
  };

  const handleTunToggle = async (enabled: boolean) => {
    try {
      await toggleTun(enabled);
    } catch (error) {
      console.error('Failed to toggle TUN mode:', error);
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <Globe className="h-6 w-6 text-primary" />
              <h1 className="text-2xl font-bold">Mihomo Manager</h1>
            </div>
            
            <div className="flex items-center space-x-4">
              {/* TUN Mode Toggle */}
              <div className="flex items-center space-x-2">
                <Zap className="h-4 w-4" />
                <span className="text-sm">TUN Mode</span>
                <Switch
                  checked={config?.tun?.enable || false}
                  onCheckedChange={handleTunToggle}
                  disabled={!mihomoStatus.isRunning}
                />
              </div>

              {/* Service Control */}
              <div className="flex items-center space-x-2">
                <Badge 
                  variant={mihomoStatus.isRunning ? "default" : "secondary"}
                  className="flex items-center space-x-1"
                >
                  <Activity className="h-3 w-3" />
                  <span>{mihomoStatus.isRunning ? 'Running' : 'Stopped'}</span>
                </Badge>
                
                {mihomoStatus.isRunning ? (
                  <Button 
                    variant="outline" 
                    size="sm" 
                    onClick={handleStop}
                    className="flex items-center space-x-1"
                  >
                    <Square className="h-3 w-3" />
                    <span>Stop</span>
                  </Button>
                ) : (
                  <Button 
                    size="sm" 
                    onClick={handleStart}
                    className="flex items-center space-x-1"
                  >
                    <Play className="h-3 w-3" />
                    <span>Start</span>
                  </Button>
                )}
              </div>
            </div>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-4 py-6">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="dashboard" className="flex items-center space-x-2">
              <Activity className="h-4 w-4" />
              <span>Dashboard</span>
            </TabsTrigger>
            <TabsTrigger value="subscriptions" className="flex items-center space-x-2">
              <Globe className="h-4 w-4" />
              <span>Subscriptions</span>
            </TabsTrigger>
            <TabsTrigger value="proxies" className="flex items-center space-x-2">
              <Zap className="h-4 w-4" />
              <span>Proxies</span>
            </TabsTrigger>
            <TabsTrigger value="config" className="flex items-center space-x-2">
              <Settings className="h-4 w-4" />
              <span>Config</span>
            </TabsTrigger>
          </TabsList>

          <div className="mt-6">
            <TabsContent value="dashboard" className="space-y-6">
              <Dashboard 
                status={mihomoStatus}
                config={config}
                messages={messages}
              />
            </TabsContent>

            <TabsContent value="subscriptions" className="space-y-6">
              <SubscriptionManager />
            </TabsContent>

            <TabsContent value="proxies" className="space-y-6">
              <ProxyManager 
                isRunning={mihomoStatus.isRunning}
              />
            </TabsContent>

            <TabsContent value="config" className="space-y-6">
              <ConfigManager />
            </TabsContent>
          </div>
        </Tabs>
      </main>

      {/* Real-time notifications */}
      {messages.length > 0 && (
        <div className="fixed bottom-4 right-4 max-w-sm">
          {messages.slice(-3).map((message, index) => (
            <Card key={index} className="mb-2 bg-card border-l-4 border-l-primary">
              <CardHeader className="pb-2">
                <div className="flex items-center space-x-2">
                  <AlertCircle className="h-4 w-4 text-primary" />
                  <CardTitle className="text-sm capitalize">
                    {message.type.replace('_', ' ')}
                  </CardTitle>
                </div>
              </CardHeader>
              <CardContent>
                <p className="text-xs text-muted-foreground">
                  {typeof message.data === 'string' ? message.data : JSON.stringify(message.data)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

export default App;
