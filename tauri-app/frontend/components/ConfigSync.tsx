import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  Alert,
  AlertTitle,
  CircularProgress,
  Chip,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  LinearProgress,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Grid,
  Paper
} from '@mui/material';
import {
  CheckCircle as CheckIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  Sync as SyncIcon,
  RestartAlt as ResetIcon,
  Assessment as ReportIcon,
  CloudUpload as MigrateIcon,
  ExpandMore as ExpandMoreIcon,
  Computer as PlatformIcon,
  Security as SecurityIcon,
  Speed as PerformanceIcon,
  Dns as DnsIcon
} from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/tauri';

interface ConfigStatus {
  is_standardized: boolean;
  version: string;
  issues_count: number;
  last_sync: string | null;
  platform: string;
}

interface ConfigSyncResult {
  success: boolean;
  changes: string[];
  issues: string[];
  backup_created: boolean;
  config_path: string;
}

interface HealthCheckResult {
  health_score: number;
  status: ConfigStatus;
  issues: string[];
  recommendations: string[];
  timestamp: string;
}

const ConfigSync: React.FC = () => {
  const [status, setStatus] = useState<ConfigStatus | null>(null);
  const [healthData, setHealthData] = useState<HealthCheckResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [showReport, setShowReport] = useState(false);
  const [report, setReport] = useState<string>('');
  const [syncResult, setSyncResult] = useState<ConfigSyncResult | null>(null);

  // 加载配置状态
  const loadConfigStatus = async () => {
    try {
      const configStatus = await invoke<ConfigStatus>('check_config_status');
      const healthCheck = await invoke<HealthCheckResult>('config_health_check');
      
      setStatus(configStatus);
      setHealthData(healthCheck);
    } catch (error) {
      console.error('加载配置状态失败:', error);
    }
  };

  // 标准化配置
  const handleStandardizeConfig = async () => {
    setLoading(true);
    try {
      const result = await invoke<ConfigSyncResult>('standardize_config');
      setSyncResult(result);
      await loadConfigStatus(); // 重新加载状态
    } catch (error) {
      console.error('标准化配置失败:', error);
    } finally {
      setLoading(false);
    }
  };

  // 从模板重置配置
  const handleResetFromTemplate = async () => {
    setLoading(true);
    try {
      const result = await invoke<ConfigSyncResult>('reset_config_from_template');
      setSyncResult(result);
      await loadConfigStatus();
    } catch (error) {
      console.error('重置配置失败:', error);
    } finally {
      setLoading(false);
    }
  };

  // 生成配置报告
  const handleGenerateReport = async () => {
    setLoading(true);
    try {
      const reportContent = await invoke<string>('generate_config_report');
      setReport(reportContent);
      setShowReport(true);
    } catch (error) {
      console.error('生成报告失败:', error);
    } finally {
      setLoading(false);
    }
  };

  // 获取健康状态颜色
  const getHealthColor = (score: number) => {
    if (score >= 90) return 'success';
    if (score >= 70) return 'warning';
    return 'error';
  };

  // 获取状态图标
  const getStatusIcon = (isStandardized: boolean, issuesCount: number) => {
    if (isStandardized && issuesCount === 0) {
      return <CheckIcon color="success" />;
    }
    if (issuesCount > 0) {
      return <WarningIcon color="warning" />;
    }
    return <ErrorIcon color="error" />;
  };

  useEffect(() => {
    loadConfigStatus();
  }, []);

  if (!status || !healthData) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box sx={{ p: 3, maxWidth: 1200, mx: 'auto' }}>
      {/* 健康状态总览 */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Grid container spacing={3} alignItems="center">
            <Grid item xs={12} md={8}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
                {getStatusIcon(status.is_standardized, status.issues_count)}
                <Typography variant="h5">
                  配置健康状态
                </Typography>
                <Chip 
                  label={`${healthData.health_score}分`}
                  color={getHealthColor(healthData.health_score)}
                  size="small"
                />
              </Box>
              
              <Typography variant="body2" color="textSecondary" sx={{ mb: 1 }}>
                平台: {status.platform} | 版本: {status.version}
              </Typography>
              
              {status.last_sync && (
                <Typography variant="body2" color="textSecondary">
                  上次标准化: {new Date(status.last_sync).toLocaleString()}
                </Typography>
              )}
            </Grid>
            
            <Grid item xs={12} md={4}>
              <Box sx={{ position: 'relative', display: 'inline-flex' }}>
                <CircularProgress
                  variant="determinate"
                  value={healthData.health_score}
                  size={80}
                  thickness={4}
                  color={getHealthColor(healthData.health_score)}
                />
                <Box
                  sx={{
                    top: 0,
                    left: 0,
                    bottom: 0,
                    right: 0,
                    position: 'absolute',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  <Typography variant="h6" component="div" color="textSecondary">
                    {healthData.health_score}%
                  </Typography>
                </Box>
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {/* 快速操作按钮 */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2 }}>🔧 配置管理操作</Typography>
          <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
            <Button
              variant="contained"
              startIcon={<SyncIcon />}
              onClick={handleStandardizeConfig}
              disabled={loading}
              color="primary"
            >
              标准化配置
            </Button>
            
            <Button
              variant="outlined"
              startIcon={<ResetIcon />}
              onClick={handleResetFromTemplate}
              disabled={loading}
              color="warning"
            >
              重置为模板
            </Button>
            
            <Button
              variant="outlined"
              startIcon={<ReportIcon />}
              onClick={handleGenerateReport}
              disabled={loading}
            >
              生成报告
            </Button>
            
            <Button
              variant="outlined"
              startIcon={<MigrateIcon />}
              disabled={loading}
              onClick={() => {/* TODO: 实现配置导入 */}}
            >
              导入配置
            </Button>
          </Box>
        </CardContent>
      </Card>

      {/* 配置问题和建议 */}
      {(healthData.issues.length > 0 || healthData.recommendations.length > 0) && (
        <Grid container spacing={2} sx={{ mb: 3 }}>
          {healthData.issues.length > 0 && (
            <Grid item xs={12} md={6}>
              <Alert severity="warning">
                <AlertTitle>配置问题 ({healthData.issues.length})</AlertTitle>
                <List dense>
                  {healthData.issues.slice(0, 3).map((issue, index) => (
                    <ListItem key={index} sx={{ py: 0 }}>
                      <ListItemIcon sx={{ minWidth: 32 }}>
                        <WarningIcon fontSize="small" />
                      </ListItemIcon>
                      <ListItemText 
                        primary={issue}
                        primaryTypographyProps={{ fontSize: '0.875rem' }}
                      />
                    </ListItem>
                  ))}
                  {healthData.issues.length > 3 && (
                    <Typography variant="caption" sx={{ pl: 2 }}>
                      还有 {healthData.issues.length - 3} 个问题...
                    </Typography>
                  )}
                </List>
              </Alert>
            </Grid>
          )}
          
          <Grid item xs={12} md={6}>
            <Alert severity="info">
              <AlertTitle>优化建议</AlertTitle>
              <List dense>
                {healthData.recommendations.slice(0, 3).map((rec, index) => (
                  <ListItem key={index} sx={{ py: 0 }}>
                    <ListItemIcon sx={{ minWidth: 32 }}>
                      <CheckIcon fontSize="small" />
                    </ListItemIcon>
                    <ListItemText 
                      primary={rec}
                      primaryTypographyProps={{ fontSize: '0.875rem' }}
                    />
                  </ListItem>
                ))}
              </List>
            </Alert>
          </Grid>
        </Grid>
      )}

      {/* 配置特性分析 */}
      <Grid container spacing={2} sx={{ mb: 3 }}>
        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 2, textAlign: 'center' }}>
            <DnsIcon color="primary" sx={{ fontSize: 40, mb: 1 }} />
            <Typography variant="h6">DNS 优化</Typography>
            <Typography variant="body2" color="textSecondary">
              三层架构
            </Typography>
          </Paper>
        </Grid>
        
        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 2, textAlign: 'center' }}>
            <PerformanceIcon color="success" sx={{ fontSize: 40, mb: 1 }} />
            <Typography variant="h6">性能配置</Typography>
            <Typography variant="body2" color="textSecondary">
              优化加速
            </Typography>
          </Paper>
        </Grid>
        
        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 2, textAlign: 'center' }}>
            <SecurityIcon color="warning" sx={{ fontSize: 40, mb: 1 }} />
            <Typography variant="h6">路由规则</Typography>
            <Typography variant="body2" color="textSecondary">
              智能分流
            </Typography>
          </Paper>
        </Grid>
        
        <Grid item xs={12} sm={6} md={3}>
          <Paper sx={{ p: 2, textAlign: 'center' }}>
            <PlatformIcon color="info" sx={{ fontSize: 40, mb: 1 }} />
            <Typography variant="h6">跨平台</Typography>
            <Typography variant="body2" color="textSecondary">
              统一配置
            </Typography>
          </Paper>
        </Grid>
      </Grid>

      {/* 操作结果对话框 */}
      <Dialog 
        open={Boolean(syncResult)} 
        onClose={() => setSyncResult(null)}
        maxWidth="md"
        fullWidth
      >
        <DialogTitle>
          {syncResult?.success ? '✅ 操作成功' : '❌ 操作失败'}
        </DialogTitle>
        <DialogContent>
          {syncResult?.backup_created && (
            <Alert severity="info" sx={{ mb: 2 }}>
              已自动创建配置备份
            </Alert>
          )}
          
          {syncResult?.changes && syncResult.changes.length > 0 && (
            <Accordion defaultExpanded>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography>配置变更 ({syncResult.changes.length})</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <List>
                  {syncResult.changes.map((change, index) => (
                    <ListItem key={index}>
                      <ListItemIcon>
                        <CheckIcon color="success" />
                      </ListItemIcon>
                      <ListItemText primary={change} />
                    </ListItem>
                  ))}
                </List>
              </AccordionDetails>
            </Accordion>
          )}
          
          {syncResult?.issues && syncResult.issues.length > 0 && (
            <Accordion>
              <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                <Typography>剩余问题 ({syncResult.issues.length})</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <List>
                  {syncResult.issues.map((issue, index) => (
                    <ListItem key={index}>
                      <ListItemIcon>
                        <WarningIcon color="warning" />
                      </ListItemIcon>
                      <ListItemText primary={issue} />
                    </ListItem>
                  ))}
                </List>
              </AccordionDetails>
            </Accordion>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSyncResult(null)}>关闭</Button>
        </DialogActions>
      </Dialog>

      {/* 配置报告对话框 */}
      <Dialog 
        open={showReport} 
        onClose={() => setShowReport(false)}
        maxWidth="lg"
        fullWidth
      >
        <DialogTitle>📊 配置分析报告</DialogTitle>
        <DialogContent>
          <Box 
            component="pre" 
            sx={{ 
              whiteSpace: 'pre-wrap', 
              fontSize: '0.875rem',
              backgroundColor: 'background.default',
              p: 2,
              borderRadius: 1,
              overflow: 'auto',
              maxHeight: '60vh'
            }}
          >
            {report}
          </Box>
        </DialogContent>
        <DialogActions>
          <Button 
            onClick={() => {
              navigator.clipboard.writeText(report);
            }}
          >
            复制报告
          </Button>
          <Button onClick={() => setShowReport(false)}>关闭</Button>
        </DialogActions>
      </Dialog>

      {loading && <LinearProgress sx={{ position: 'fixed', top: 0, left: 0, right: 0, zIndex: 9999 }} />}
    </Box>
  );
};

export default ConfigSync;
