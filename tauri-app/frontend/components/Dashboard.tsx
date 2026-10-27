import React from "react";
import { Box, Grid } from "@mui/material";
import ServiceControl from "./ServiceControl";
import IPInfoCard from "./IPInfoCard";
import SystemStatusCard from "./SystemStatusCard";

interface DashboardProps {
  isRunning: boolean;
  onStatusChange: () => void;
  showNotification: (message: string, severity?: "success" | "error" | "info" | "warning") => void;
}

const Dashboard: React.FC<DashboardProps> = React.memo(
  ({ isRunning, onStatusChange, showNotification }) => {
    return (
      <Box sx={{ width: "100%" }}>
        <Grid container spacing={2.5}>
          {/* Left Column */}
          <Grid item xs={12} lg={8}>
            <Grid container spacing={2.5}>
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
                <SystemStatusCard isRunning={isRunning} showNotification={showNotification} />
              </Grid>
            </Grid>
          </Grid>

          {/* Right Column - IP Information */}
          <Grid item xs={12} lg={4}>
            <IPInfoCard isRunning={isRunning} showNotification={showNotification} />
          </Grid>
        </Grid>
      </Box>
    );
  }
);

Dashboard.displayName = "Dashboard";

export default Dashboard;
