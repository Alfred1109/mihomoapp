import React from 'react';
import { Box, IconButton, Typography } from '@mui/material';
import { Minimize, CropSquare, Close, Remove } from '@mui/icons-material';
import { appWindow } from '@tauri-apps/api/window';

interface TitleBarProps {
  title?: string;
  children?: React.ReactNode;
}

const TitleBar: React.FC<TitleBarProps> = ({ title = 'Mihomo Manager', children }) => {
  const handleMinimize = () => appWindow.minimize();
  const handleMaximize = () => appWindow.toggleMaximize();
  const handleClose = () => appWindow.close();

  return (
    <Box
      sx={{
        height: 38,
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
        <Box
          component="img"
          src="/icons/32x32.png"
          alt="logo"
          sx={{ width: 18, height: 18 }}
          onError={(e: React.SyntheticEvent<HTMLImageElement>) => {
            e.currentTarget.style.display = 'none';
          }}
        />
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
          sx={{
            borderRadius: 0,
            width: 46,
            height: 38,
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
          sx={{
            borderRadius: 0,
            width: 46,
            height: 38,
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
          sx={{
            borderRadius: 0,
            width: 46,
            height: 38,
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
