import React from 'react';
import { Box, Grid } from '@mui/material';
import ServiceControl from './ServiceControl';
import IPInfoCard from './IPInfoCard';
import SystemStatusCard from './SystemStatusCard';

interface DashboardProps {
  isRunning: boolean;
  onStatusChange: (status: boolean) => void;
  showNotification: (message: string, severity?: 'success' | 'error' | 'info' | 'warning') => void;
}

const Dashboard: React.FC<DashboardProps> = ({ isRunning, onStatusChange, showNotification }) => {
  return (
    <Box>
      <Grid container spacing={3}>
        {/* Left Column */}
        <Grid item xs={12} md={8}>
          <Grid container spacing={3}>
            {/* Service Control */}
            <Grid item xs={12}>
              <ServiceControl
                isRunning={isRunning}
                onStatusChange={onStatusChange}
                showNotification={showNotification}
              />
            </Grid>

            {/* System Status */}
            <Grid item xs={12}>
              <SystemStatusCard
                isRunning={isRunning}
                showNotification={showNotification}
              />
            </Grid>
          </Grid>
        </Grid>

        {/* Right Column - IP Information */}
        <Grid item xs={12} md={4}>
          <IPInfoCard
            isRunning={isRunning}
            showNotification={showNotification}
          />
        </Grid>
      </Grid>
    </Box>
  );
};

export default Dashboard;
