import React from 'react';
import { Box } from '@mui/material';

interface TabPanelProps {
  children?: React.ReactNode;
  index: number;
  value: number;
  mounted?: boolean;
  idPrefix?: string;
}

const TabPanel: React.FC<TabPanelProps> = ({ children, value, index, mounted = value === index, idPrefix = '', ...other }) => {
  return (
    <div
      role="tabpanel"
      hidden={value !== index}
      id={`${idPrefix}tabpanel-${index}`}
      aria-labelledby={`${idPrefix}tab-${index}`}
      {...other}
    >
      {mounted && <Box sx={{ p: idPrefix ? { xs: 2, md: 3 } : 0 }}>{children}</Box>}
    </div>
  );
};

export default TabPanel;
