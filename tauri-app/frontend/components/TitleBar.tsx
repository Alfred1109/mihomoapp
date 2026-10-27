import React from 'react';
import { Box, IconButton, Typography } from '@mui/material';
import { CropSquare, Close, Remove, HubRounded } from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { appWindow } from '@tauri-apps/api/window';

interface TitleBarProps {
  title?: string;
  children?: React.ReactNode;
}

const TitleBar: React.FC<TitleBarProps> = ({ title = 'Mihomo Manager', children }) => {
  const { t } = useTranslation();
  const handleMinimize = () => appWindow.minimize();
  const handleMaximize = () => appWindow.toggleMaximize();
  const handleClose = () => appWindow.hide(); // 最小化到托盘而不是退出

  return (
    <Box
      sx={{
        height: 48,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        backgroundColor: 'background.default',
        borderBottom: '1px solid',
        borderColor: 'divider',
        px: 1,
        WebkitAppRegion: 'drag',
        userSelect: 'none',
      }}
      data-tauri-drag-region
    >
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, pl: 1 }}>
        <Box sx={{ width: 25, height: 25, display: 'grid', placeItems: 'center', borderRadius: 1.5, background: 'linear-gradient(145deg, #72bcff, #4388df)', color: '#0a203b', boxShadow: '0 3px 12px rgba(71, 148, 231, 0.25)' }}>
          <HubRounded sx={{ fontSize: 17 }} />
        </Box>
        <Typography
          variant="body2"
          sx={{
            fontWeight: 600,
            fontSize: '0.8125rem',
            color: 'text.primary',
            letterSpacing: '-0.01em',
          }}
        >
          {title}
        </Typography>
      </Box>

      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0,
          WebkitAppRegion: 'no-drag',
        }}
      >
        {children}

        <IconButton
          size="small"
          onClick={handleMinimize}
          aria-label={t('app.minimize')}
          sx={{
            borderRadius: 0,
            width: 46,
            height: 48,
            color: 'text.secondary',
            '&:hover': {
              backgroundColor: 'rgba(255, 255, 255, 0.06)',
              color: 'text.primary',
            },
          }}
        >
          <Remove sx={{ fontSize: 18 }} />
        </IconButton>

        <IconButton
          size="small"
          onClick={handleMaximize}
          aria-label={t('app.maximize')}
          sx={{
            borderRadius: 0,
            width: 46,
            height: 48,
            color: 'text.secondary',
            '&:hover': {
              backgroundColor: 'rgba(255, 255, 255, 0.06)',
              color: 'text.primary',
            },
          }}
        >
          <CropSquare sx={{ fontSize: 14 }} />
        </IconButton>

        <IconButton
          size="small"
          onClick={handleClose}
          aria-label={t('app.hideWindow')}
          sx={{
            borderRadius: 0,
            width: 46,
            height: 48,
            color: 'text.secondary',
            '&:hover': {
              backgroundColor: '#c42b1c',
              color: 'white',
            },
          }}
        >
          <Close sx={{ fontSize: 18 }} />
        </IconButton>
      </Box>
    </Box>
  );
};

export default TitleBar;
