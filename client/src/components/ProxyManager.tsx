import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Activity, Zap, Globe, CheckCircle, Clock, Wifi } from 'lucide-react';
import { useMihomoAPI } from '@/hooks/useMihomoAPI';
import axios from 'axios';

interface ProxyNode {
  name: string;
  type: string;
  delay?: number;
  alive?: boolean;
}

interface ProxyGroup {
  name: string;
  type: string;
  now?: string;
  all?: string[];
  history?: Array<{ name: string; delay: number; time: string }>;
}

interface ProxyManagerProps {
  isRunning: boolean;
}

const ProxyManager: React.FC<ProxyManagerProps> = ({ isRunning }) => {
  const [proxies, setProxies] = useState<{ [key: string]: ProxyNode | ProxyGroup }>({});
  const [groups, setGroups] = useState<ProxyGroup[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedGroup, setSelectedGroup] = useState<string>('');
  
  const { switchProxy, fetchProxies } = useMihomoAPI();

  // Load proxies from API
  const loadProxies = async () => {
    if (!isRunning) return;
    
    setIsLoading(true);
    try {
      const response = await axios.get('/api/mihomo/proxies');
      setProxies(response.data.proxies || {});
      
      // Extract proxy groups
      const proxyGroups: ProxyGroup[] = [];
      Object.entries(response.data.proxies || {}).forEach(([name, proxy]: [string, any]) => {
        if (proxy.type === 'Selector' || proxy.type === 'URLTest' || proxy.type === 'Fallback') {
          proxyGroups.push({
            name,
            type: proxy.type,
            now: proxy.now,
            all: proxy.all || [],
            history: proxy.history || []
          });
        }
      });
      
      setGroups(proxyGroups);
      if (proxyGroups.length > 0 && !selectedGroup) {
        setSelectedGroup(proxyGroups[0].name);
      }
    } catch (error) {
      console.error('Failed to load proxies:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Switch proxy for selected group
  const handleSwitchProxy = async (groupName: string, proxyName: string) => {
    try {
      setIsLoading(true);
      await switchProxy(groupName, proxyName);
      await loadProxies(); // Refresh data
    } catch (error) {
      console.error('Failed to switch proxy:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Test proxy delay
  const testProxyDelay = async (proxyName: string) => {
    try {
      const response = await axios.get(`/api/mihomo/proxies/${proxyName}/delay?timeout=3000&url=http://www.gstatic.com/generate_204`);
      return response.data.delay;
    } catch (error) {
      return -1; // Timeout or error
    }
  };

  useEffect(() => {
    loadProxies();
    
    if (isRunning) {
      const interval = setInterval(loadProxies, 10000); // Refresh every 10 seconds
      return () => clearInterval(interval);
    }
  }, [isRunning]);

  if (!isRunning) {
    return (
      <div className="space-y-6">
        <Card className="text-center py-12">
          <CardContent>
            <Wifi className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">Proxy Service Not Running</h3>
            <p className="text-muted-foreground mb-4">
              Start the mihomo service to manage proxy nodes and groups
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
          <h2 className="text-2xl font-bold tracking-tight">Proxy Manager</h2>
          <p className="text-muted-foreground">
            Switch between proxy nodes and manage proxy groups
          </p>
        </div>
        <Button onClick={loadProxies} disabled={isLoading}>
          <Activity className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
          Refresh
        </Button>
      </div>

      {/* Proxy Groups Overview */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {groups.map((group) => (
          <Card key={group.name} className="relative">
            <CardHeader className="pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-base">{group.name}</CardTitle>
                <Badge variant="outline" className="text-xs">
                  {group.type}
                </Badge>
              </div>
              <CardDescription>
                Current: {group.now || 'None'}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Available Nodes:</span>
                  <span className="font-medium">{group.all?.length || 0}</span>
                </div>
                <div className="flex items-center space-x-2">
                  <Globe className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm text-muted-foreground truncate">
                    {group.now || 'No proxy selected'}
                  </span>
                  {group.now && (
                    <CheckCircle className="h-4 w-4 text-green-500" />
                  )}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Detailed Proxy Management */}
      {groups.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Node Management</CardTitle>
            <CardDescription>
              Switch proxy nodes for different proxy groups
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Group Selector */}
            <div className="flex items-center space-x-4">
              <label className="text-sm font-medium min-w-fit">Proxy Group:</label>
              <Select value={selectedGroup} onValueChange={setSelectedGroup}>
                <SelectTrigger className="w-64">
                  <SelectValue placeholder="Select a proxy group" />
                </SelectTrigger>
                <SelectContent>
                  {groups.map((group) => (
                    <SelectItem key={group.name} value={group.name}>
                      {group.name} ({group.type})
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Node List for Selected Group */}
            {selectedGroup && (() => {
              const group = groups.find(g => g.name === selectedGroup);
              if (!group || !group.all) return null;

              return (
                <div className="space-y-3">
                  <h4 className="text-sm font-medium">Available Nodes for {selectedGroup}:</h4>
                  <div className="grid gap-3">
                    {group.all.map((nodeName) => {
                      const node = proxies[nodeName] as ProxyNode;
                      const isActive = group.now === nodeName;
                      const history = group.history?.find(h => h.name === nodeName);
                      
                      return (
                        <div 
                          key={nodeName}
                          className={`flex items-center justify-between p-3 rounded-md border ${
                            isActive ? 'border-primary bg-primary/5' : 'border-border'
                          }`}
                        >
                          <div className="flex items-center space-x-3">
                            <div className={`w-2 h-2 rounded-full ${
                              isActive ? 'bg-green-500' : node?.alive !== false ? 'bg-green-400' : 'bg-red-400'
                            }`} />
                            <div>
                              <p className="font-medium">{nodeName}</p>
                              <p className="text-xs text-muted-foreground">
                                {node?.type || 'Unknown'} 
                                {history && (
                                  <span className="ml-2">
                                    • Delay: {history.delay}ms
                                  </span>
                                )}
                              </p>
                            </div>
                          </div>
                          
                          <div className="flex items-center space-x-2">
                            {isActive && (
                              <Badge variant="default" className="text-xs">
                                Active
                              </Badge>
                            )}
                            <Button
                              variant={isActive ? "secondary" : "outline"}
                              size="sm"
                              onClick={() => handleSwitchProxy(selectedGroup, nodeName)}
                              disabled={isLoading || isActive}
                            >
                              {isActive ? 'Current' : 'Switch'}
                            </Button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })()}
          </CardContent>
        </Card>
      )}

      {/* Connection Statistics */}
      {selectedGroup && (() => {
        const group = groups.find(g => g.name === selectedGroup);
        if (!group?.history?.length) return null;

        return (
          <Card>
            <CardHeader>
              <CardTitle>Connection History</CardTitle>
              <CardDescription>
                Recent connection attempts and delays for {selectedGroup}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                {group.history.slice(0, 10).map((item, index) => (
                  <div key={index} className="flex items-center justify-between py-2 border-b last:border-b-0">
                    <div className="flex items-center space-x-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-medium">{item.name}</span>
                    </div>
                    <div className="flex items-center space-x-2">
                      <span className="text-sm text-muted-foreground">
                        {new Date(item.time).toLocaleTimeString()}
                      </span>
                      <Badge 
                        variant={item.delay < 100 ? "default" : item.delay < 300 ? "secondary" : "destructive"}
                        className="text-xs"
                      >
                        {item.delay}ms
                      </Badge>
                    </div>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        );
      })()}

      {/* No Proxies Available */}
      {groups.length === 0 && !isLoading && (
        <Card className="text-center py-12">
          <CardContent>
            <Zap className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No Proxy Groups Available</h3>
            <p className="text-muted-foreground mb-4">
              Configure proxy subscriptions first to manage proxy nodes
            </p>
            <Button variant="outline">
              <Globe className="mr-2 h-4 w-4" />
              Add Subscriptions
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

export default ProxyManager;
