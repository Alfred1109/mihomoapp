import React, { useState, useEffect, useCallback } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  IconButton,
  CircularProgress,
  Chip,
  Divider,
  Alert,
  Tooltip,
} from '@mui/material';
import {
  Refresh,
  Language,
  LocationOn,
  Business,
  Public,
  ErrorOutline,
  Warning,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';
import { useTranslation } from 'react-i18next';
import { isTauriEnv } from '../utils/tauri';
import type { IPInfo } from '../types';

interface ExtendedIPInfo extends IPInfo {
  proxy_status?: 'proxied' | 'direct';
  country_code?: string;
  stale?: boolean;
}

interface IPInfoCardProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const IPInfoCard: React.FC<IPInfoCardProps> = React.memo(({ isRunning, showNotification }) => {
  void showNotification;
  const { t } = useTranslation();
  const [ipInfo, setIpInfo] = useState<ExtendedIPInfo | null>(null);
  const [ipLoading, setIpLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  const loadIpInfo = useCallback(async (forceRefresh = false) => {
    if (!isTauriEnv()) return;
    
    const cachedData = localStorage.getItem('ipInfo');
    const cachedTime = localStorage.getItem('ipInfoTime');
    
    if (!forceRefresh && cachedData && cachedTime) {
      const timeDiff = Date.now() - parseInt(cachedTime);
      if (timeDiff < 5 * 60 * 1000) {
        setIpInfo(JSON.parse(cachedData));
        setLoadError(null);
        return;
      }
    }
    
    setIpLoading(true);
    setLoadError(null);
    
    try {
      const data = await invoke<ExtendedIPInfo>('get_current_ip');
      setIpInfo(data);
      setRetryCount(0);
      localStorage.setItem('ipInfo', JSON.stringify(data));
      localStorage.setItem('ipInfoTime', Date.now().toString());
    } catch (error: unknown) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      
      if (cachedData) {
        const cached = JSON.parse(cachedData) as ExtendedIPInfo;
        cached.stale = true;
        setIpInfo(cached);
      }
      
      if (errorMsg.includes('Failed to get IP')) {
        setLoadError(t('ipInfo.networkError'));
      } else {
        setLoadError(`${t('ipInfo.error')}: ${errorMsg}`);
      }
    } finally {
      setIpLoading(false);
    }
  }, [t]);

  useEffect(() => {
    if (isRunning) {
      const timer = setTimeout(() => {
        void loadIpInfo(true);
      }, 3000);
      return () => clearTimeout(timer);
    } else {
      void loadIpInfo(true);
    }
  }, [isRunning, loadIpInfo]);

  const getCountryFlag = (countryCode?: string) => {
    if (!countryCode || countryCode.length !== 2) return '🌍';
    const codePoints = countryCode
      .toUpperCase()
      .split('')
      .map(char => 127397 + char.charCodeAt(0));
    return String.fromCodePoint(...codePoints);
  };

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6">
            IP 信息
          </Typography>
          <IconButton onClick={() => void loadIpInfo(true)} disabled={ipLoading} size="small">
            <Refresh />
          </IconButton>
        </Box>

        {/* Error Alert */}
        {loadError && !ipLoading && (
          <Alert 
            severity="warning" 
            sx={{ mb: 2 }}
            icon={<Warning />}
          >
            {loadError}
            {retryCount > 0 && retryCount < 3 && (
              <Typography variant="caption" display="block">
                正在重试... ({retryCount}/3)
              </Typography>
            )}
          </Alert>
        )}

        {ipLoading ? (
          <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', py: 4 }}>
            <CircularProgress size={32} />
            <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
              正在检测IP地址...
            </Typography>
          </Box>
        ) : ipInfo ? (
          <Box>
            {/* Stale data warning */}
            {ipInfo.stale && (
              <Alert severity="info" sx={{ mb: 2 }} icon={<ErrorOutline />}>
                显示的是缓存数据，可能不是最新的
              </Alert>
            )}

            {/* IP Address */}
            <Box sx={{ mb: 2 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                <Public color="primary" />
                <Typography variant="subtitle2" color="text.secondary">
                  IP 地址
                </Typography>
              </Box>
              <Typography variant="h6" sx={{ fontFamily: 'monospace' }}>
                {ipInfo.query || ipInfo.ip || 'N/A'}
              </Typography>
            </Box>

            <Divider sx={{ my: 2 }} />

            {/* Location */}
            <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Language fontSize="small" color="action" />
                <Typography variant="body2" color="text.secondary">
                  国家：
                </Typography>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                  <Typography variant="body2">
                    {getCountryFlag(ipInfo.countryCode || ipInfo.country_code)}
                  </Typography>
                  <Typography variant="body2">
                    {ipInfo.country || 'N/A'}
                  </Typography>
                </Box>
              </Box>

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <LocationOn fontSize="small" color="action" />
                <Typography variant="body2" color="text.secondary">
                  地区：
                </Typography>
                <Typography variant="body2">
                  {ipInfo.regionName || ipInfo.region || 'N/A'}
                </Typography>
              </Box>

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <LocationOn fontSize="small" color="action" />
                <Typography variant="body2" color="text.secondary">
                  城市：
                </Typography>
                <Typography variant="body2">
                  {ipInfo.city || 'N/A'}
                </Typography>
              </Box>

              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Business fontSize="small" color="action" />
                <Typography variant="body2" color="text.secondary">
                  ISP：
                </Typography>
                <Typography variant="body2" sx={{ flex: 1, wordBreak: 'break-word' }}>
                  {ipInfo.isp || ipInfo.org || 'N/A'}
                </Typography>
              </Box>
            </Box>

            {/* Status Chips */}
            <Box sx={{ mt: 2, display: 'flex', gap: 1, flexWrap: 'wrap' }}>
              <Chip
                label={isRunning ? '服务运行中' : '服务未运行'}
                color={isRunning ? 'success' : 'default'}
                size="small"
              />
              {ipInfo.proxy_status && (
                <Tooltip 
                  title={
                    ipInfo.proxy_status === 'proxied' 
                      ? '您的IP已通过代理服务器，可以正常访问国际网站' 
                      : '您的IP是直连状态，代理可能未生效。请确保TUN模式已启用并重启服务。'
                  }
                >
                  <Chip
                    label={ipInfo.proxy_status === 'proxied' ? '已通过代理' : '直连（代理未生效）'}
                    color={ipInfo.proxy_status === 'proxied' ? 'primary' : 'warning'}
                    size="small"
                    variant="outlined"
                    icon={ipInfo.proxy_status === 'proxied' ? undefined : <Warning fontSize="small" />}
                  />
                </Tooltip>
              )}
            </Box>

            {/* TUN mode hint when proxy not working */}
            {isRunning && ipInfo.proxy_status === 'direct' && (
              <Alert severity="info" sx={{ mt: 2 }} variant="outlined">
                <Typography variant="caption">
                  提示：如果需要全局代理（让所有应用都走代理），请启用 TUN 模式并确保程序以管理员身份运行。
                </Typography>
              </Alert>
            )}
          </Box>
        ) : (
          <Box sx={{ textAlign: 'center', py: 4 }}>
            <ErrorOutline color="action" sx={{ fontSize: 40, mb: 1 }} />
            <Typography variant="body2" color="text.secondary">
              无法获取 IP 信息
            </Typography>
            <Typography variant="caption" color="text.secondary">
              请检查网络连接或点击刷新按钮重试
            </Typography>
          </Box>
        )}
      </CardContent>
    </Card>
  );
});

IPInfoCard.displayName = 'IPInfoCard';

export default IPInfoCard;
