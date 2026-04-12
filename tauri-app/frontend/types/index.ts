export interface MihomoStatusEvent {
  running: boolean;
  process_id: number | null;
  timestamp: number;
}

export interface ConfigChangeEvent {
  config_path: string;
  timestamp: number;
}

export interface ProxyChangeEvent {
  group_name: string;
  proxy_name: string;
  timestamp: number;
}

export interface SubscriptionUpdateEvent {
  subscription_id: string;
  status: string;
  proxy_count: number;
  timestamp: number;
}

export interface MihomoStatus {
  running: boolean;
  processId: number | null;
  timestamp: number;
}

export interface Subscription {
  id: string;
  name: string;
  url: string;
  user_agent?: string | null;
  use_proxy: boolean;
  created_at: string;
  last_updated: string;
  proxy_count: number;
  status: SubscriptionStatus;
  last_error?: string | null;
}

export type SubscriptionStatus = 'Active' | 'Error' | 'Updating';

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface RestoreResult {
  success: boolean;
  service_running: boolean;
  message: string;
  requires_restart: boolean;
}

export interface ProxyGroup {
  name: string;
  type: string;
  now?: string;
  all: string[];
  history?: ProxyHistory[];
}

export interface ProxyNode {
  name: string;
  type: string;
  server?: string;
  port?: number;
  delay?: number;
  alive?: boolean;
  history?: ProxyHistory[];
}

export interface ProxyHistory {
  time: string;
  delay: number;
  name?: string;
}

export interface ProxiesResponse {
  proxies: Record<string, ProxyNode | ProxyGroup>;
}

export interface IPInfo {
  ip?: string;
  query?: string;
  country?: string;
  countryCode?: string;
  city?: string;
  regionName?: string;
  region?: string;
  isp?: string;
  org?: string;
  as?: string;
}

export interface SpeedTestResult {
  name: string;
  delay: number;
  success: boolean;
  error?: string;
}

export interface ConfigValue {
  port?: number;
  'socks-port'?: number;
  'mixed-port'?: number;
  'allow-lan'?: boolean;
  mode?: string;
  'log-level'?: string;
  'external-controller'?: string;
  dns?: DnsConfig;
  tun?: TunConfig;
  proxies?: ProxyNode[];
  'proxy-groups'?: ProxyGroup[];
  rules?: string[];
  [key: string]: unknown;
}

export interface DnsConfig {
  enable?: boolean;
  'prefer-h3'?: boolean;
  ipv6?: boolean;
  listen?: string;
  'enhanced-mode'?: string;
  'fake-ip-range'?: string;
  'fake-ip-filter'?: string[];
  'default-nameserver'?: string[];
  nameserver?: string[];
  'proxy-server-nameserver'?: string[];
  fallback?: string[];
  'fallback-filter'?: {
    geoip?: boolean;
    'geoip-code'?: string;
    ipcidr?: string[];
  };
  'nameserver-policy'?: Record<string, string | string[]>;
}

export interface TunConfig {
  enable?: boolean;
  stack?: string;
  'device-name'?: string;
  'auto-route'?: boolean;
  'auto-detect-interface'?: boolean;
  'dns-hijack'?: string[];
  mtu?: number;
}

export interface ApiError {
  code: string;
  message: string;
  details?: unknown;
}

export type UnlistenFn = () => void;
