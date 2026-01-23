import React, { useState, useEffect } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Box,
  IconButton,
  CircularProgress,
  Chip,
  Divider,
} from '@mui/material';
import {
  Refresh,
  Language,
  LocationOn,
  Business,
  Public,
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface IPInfoCardProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const IPInfoCard: React.FC<IPInfoCardProps> = ({ isRunning, showNotification }) => {
  const [ipInfo, setIpInfo] = useState<any>(null);
  const [ipLoading, setIpLoading] = useState(false);

  useEffect(() => {
    loadIpInfo();
  }, [isRunning]);

  const loadIpInfo = async (forceRefresh = false) => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_IPC__) return;
    
    // 检查缓存（5分钟内不重复请求）
    const cachedData = localStorage.getItem('ipInfo');
    const cachedTime = localStorage.getItem('ipInfoTime');
    
    if (!forceRefresh && cachedData && cachedTime) {
      const timeDiff = Date.now() - parseInt(cachedTime);
      if (timeDiff < 5 * 60 * 1000) {
        setIpInfo(JSON.parse(cachedData));
        return;
      }
    }
    
    setIpLoading(true);
    try {
      const data = await invoke<any>('get_current_ip');
      setIpInfo(data);
      localStorage.setItem('ipInfo', JSON.stringify(data));
      localStorage.setItem('ipInfoTime', Date.now().toString());
    } catch (error) {
      console.error('Failed to get IP info:', error);
      if (cachedData) {
        setIpInfo(JSON.parse(cachedData));
      }
    } finally {
      setIpLoading(false);
    }
  };

  const getCountryFlag = (countryCode: string) => {
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
          <IconButton onClick={() => loadIpInfo(true)} disabled={ipLoading} size="small">
            <Refresh />
          </IconButton>
        </Box>

        {ipLoading ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', py: 4 }}>
            <CircularProgress />
          </Box>
        ) : ipInfo ? (
          <Box>
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

            {/* Status Chip */}
            <Box sx={{ mt: 2 }}>
              <Chip
                label={isRunning ? '代理已启用' : '直连模式'}
                color={isRunning ? 'success' : 'default'}
                size="small"
              />
            </Box>
          </Box>
        ) : (
          <Box sx={{ textAlign: 'center', py: 4 }}>
            <Typography variant="body2" color="text.secondary">
              无法获取 IP 信息
            </Typography>
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

export default IPInfoCard;
