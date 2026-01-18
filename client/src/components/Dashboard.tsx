import React from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Activity, Wifi, Zap, Settings, Upload, Download, Clock, Server } from 'lucide-react';
import { formatBytes, formatDuration } from '@/lib/utils';

interface DashboardProps {
  status: any;
  config: any;
  messages: any[];
}

const Dashboard: React.FC<DashboardProps> = ({ status, config, messages }) => {
  return (
    <div className="grid gap-6">
      {/* Service Status Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Service Status</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="flex items-center space-x-2">
              <Badge 
                variant={status?.isRunning ? "default" : "secondary"}
                className="text-xs"
              >
                {status?.isRunning ? 'Running' : 'Stopped'}
              </Badge>
              {status?.processId && (
                <span className="text-xs text-muted-foreground">
                  PID: {status.processId}
                </span>
              )}
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              {status?.isRunning ? 'Service is active' : 'Service is inactive'}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Proxy Mode</CardTitle>
            <Wifi className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold capitalize">
              {config?.mode || 'Rule'}
            </div>
            <p className="text-xs text-muted-foreground">
              Current proxy mode
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">TUN Mode</CardTitle>
            <Zap className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="flex items-center space-x-2">
              <Badge 
                variant={config?.tun?.enable ? "default" : "secondary"}
                className="text-xs"
              >
                {config?.tun?.enable ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mt-1">
              System-wide proxy
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Configuration</CardTitle>
            <Settings className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {config ? 'Loaded' : 'Missing'}
            </div>
            <p className="text-xs text-muted-foreground">
              Config status
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Connection Information */}
      {status?.apiStatus && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center space-x-2">
              <Server className="h-5 w-5" />
              <span>Connection Information</span>
            </CardTitle>
            <CardDescription>Current proxy configuration details</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <div>
                <p className="text-sm font-medium">HTTP Port</p>
                <p className="text-2xl font-bold">{config?.port || 'N/A'}</p>
              </div>
              <div>
                <p className="text-sm font-medium">SOCKS Port</p>
                <p className="text-2xl font-bold">{config?.[`socks-port`] || 'N/A'}</p>
              </div>
              <div>
                <p className="text-sm font-medium">Mixed Port</p>
                <p className="text-2xl font-bold">{config?.[`mixed-port`] || 'N/A'}</p>
              </div>
              <div>
                <p className="text-sm font-medium">Allow LAN</p>
                <Badge variant={config?.[`allow-lan`] ? "default" : "secondary"}>
                  {config?.[`allow-lan`] ? 'Yes' : 'No'}
                </Badge>
              </div>
              <div>
                <p className="text-sm font-medium">Log Level</p>
                <p className="text-sm capitalize">{config?.[`log-level`] || 'info'}</p>
              </div>
              <div>
                <p className="text-sm font-medium">IPv6</p>
                <Badge variant={config?.ipv6 ? "default" : "secondary"}>
                  {config?.ipv6 ? 'Enabled' : 'Disabled'}
                </Badge>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* System Logs */}
      {messages.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center space-x-2">
              <Activity className="h-5 w-5" />
              <span>System Logs</span>
            </CardTitle>
            <CardDescription>Recent system events and messages</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {messages.slice(-10).reverse().map((message, index) => (
                <div 
                  key={index} 
                  className="flex items-start space-x-2 p-2 bg-muted rounded-md text-sm"
                >
                  <Badge 
                    variant={
                      message.type.includes('error') ? 'destructive' : 
                      message.type.includes('stopped') ? 'secondary' : 
                      'default'
                    }
                    className="text-xs min-w-fit"
                  >
                    {message.type.replace('_', ' ')}
                  </Badge>
                  <div className="flex-1 min-w-0">
                    <p className="text-xs text-muted-foreground font-mono break-all">
                      {typeof message.data === 'string' 
                        ? message.data.substring(0, 200) + (message.data.length > 200 ? '...' : '')
                        : JSON.stringify(message.data).substring(0, 200)
                      }
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
          <CardDescription>Common tasks and operations</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-2 md:grid-cols-2 lg:grid-cols-4">
            <Button 
              variant="outline" 
              className="justify-start"
              disabled={!status?.isRunning}
            >
              <Activity className="mr-2 h-4 w-4" />
              View Proxies
            </Button>
            <Button 
              variant="outline" 
              className="justify-start"
            >
              <Settings className="mr-2 h-4 w-4" />
              Edit Config
            </Button>
            <Button 
              variant="outline" 
              className="justify-start"
            >
              <Download className="mr-2 h-4 w-4" />
              Add Subscription
            </Button>
            <Button 
              variant="outline" 
              className="justify-start"
              disabled={!status?.isRunning}
            >
              <Upload className="mr-2 h-4 w-4" />
              Test Connection
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Status Information */}
      {!status?.isRunning && (
        <Card className="border-yellow-200 bg-yellow-50">
          <CardHeader>
            <CardTitle className="text-yellow-800">Service Not Running</CardTitle>
            <CardDescription className="text-yellow-600">
              The mihomo proxy service is currently stopped. Start the service to begin using proxy features.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <p className="text-sm text-yellow-700">
                Make sure you have:
              </p>
              <ul className="text-sm text-yellow-700 list-disc list-inside space-y-1">
                <li>Mihomo binary installed and accessible</li>
                <li>Valid configuration file</li>
                <li>Appropriate permissions (Administrator for TUN mode)</li>
              </ul>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

export default Dashboard;
