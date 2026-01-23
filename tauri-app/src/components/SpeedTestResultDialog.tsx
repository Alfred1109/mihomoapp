import React from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
  Typography,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Paper,
  Chip,
  LinearProgress,
  Alert,
} from '@mui/material';
import {
  CheckCircle,
  Error,
  Speed,
  TrendingUp,
  TrendingDown,
} from '@mui/icons-material';

interface SpeedTestResult {
  total: number;
  tested: number;
  success: number;
  results: { [key: string]: number | null };
}

interface SpeedTestResultDialogProps {
  open: boolean;
  onClose: () => void;
  result: SpeedTestResult | null;
}

const SpeedTestResultDialog: React.FC<SpeedTestResultDialogProps> = ({ open, onClose, result }) => {
  if (!result) return null;

  // 统计数据
  const successRate = ((result.success / result.total) * 100).toFixed(1);
  const failedCount = result.total - result.success;

  // 处理结果数据
  const sortedResults = Object.entries(result.results)
    .map(([name, delay]) => ({ name, delay }))
    .sort((a, b) => {
      if (a.delay === null) return 1;
      if (b.delay === null) return -1;
      return a.delay - b.delay;
    });

  // 计算平均延迟
  const validDelays = sortedResults
    .filter(r => r.delay !== null)
    .map(r => r.delay as number);
  const avgDelay = validDelays.length > 0
    ? (validDelays.reduce((a, b) => a + b, 0) / validDelays.length).toFixed(0)
    : 'N/A';

  // 最快和最慢节点
  const fastestNode = sortedResults.find(r => r.delay !== null);
  const slowestNode = [...sortedResults].reverse().find(r => r.delay !== null);

  const getDelayColor = (delay: number | null): 'success' | 'warning' | 'error' | 'default' => {
    if (!delay || delay < 0) return 'default';
    if (delay < 100) return 'success';
    if (delay < 300) return 'warning';
    return 'error';
  };

  const formatDelay = (delay: number | null): string => {
    if (delay === null) return '超时';
    return `${delay}ms`;
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Speed />
          <Typography variant="h6">批量测速结果</Typography>
        </Box>
      </DialogTitle>

      <DialogContent>
        {/* 统计摘要 */}
        <Box sx={{ mb: 3 }}>
          <Alert severity="info" sx={{ mb: 2 }}>
            <Typography variant="body2">
              <strong>测速完成！</strong> 共测试 {result.total} 个节点，
              成功 {result.success} 个，失败 {failedCount} 个
            </Typography>
          </Alert>

          <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
            <Box sx={{ flex: 1, minWidth: 200 }}>
              <Typography variant="caption" color="text.secondary">
                成功率
              </Typography>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <LinearProgress
                  variant="determinate"
                  value={parseFloat(successRate)}
                  sx={{ flex: 1, height: 8, borderRadius: 1 }}
                  color={parseFloat(successRate) > 80 ? 'success' : 'warning'}
                />
                <Typography variant="h6">
                  {successRate}%
                </Typography>
              </Box>
            </Box>

            <Box sx={{ flex: 1, minWidth: 200 }}>
              <Typography variant="caption" color="text.secondary">
                平均延迟
              </Typography>
              <Typography variant="h6">
                {avgDelay} {avgDelay !== 'N/A' && 'ms'}
              </Typography>
            </Box>
          </Box>

          {/* 最快和最慢节点 */}
          {fastestNode && slowestNode && (
            <Box sx={{ display: 'flex', gap: 2, mt: 2 }}>
              <Box sx={{ flex: 1 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
                  <TrendingUp color="success" fontSize="small" />
                  <Typography variant="caption" color="text.secondary">
                    最快节点
                  </Typography>
                </Box>
                <Typography variant="body2" noWrap>
                  {fastestNode.name}
                </Typography>
                <Chip
                  label={formatDelay(fastestNode.delay)}
                  size="small"
                  color="success"
                  sx={{ mt: 0.5 }}
                />
              </Box>

              <Box sx={{ flex: 1 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}>
                  <TrendingDown color="error" fontSize="small" />
                  <Typography variant="caption" color="text.secondary">
                    最慢节点
                  </Typography>
                </Box>
                <Typography variant="body2" noWrap>
                  {slowestNode.name}
                </Typography>
                <Chip
                  label={formatDelay(slowestNode.delay)}
                  size="small"
                  color="error"
                  sx={{ mt: 0.5 }}
                />
              </Box>
            </Box>
          )}
        </Box>

        {/* 详细结果表格 */}
        <TableContainer component={Paper} variant="outlined" sx={{ maxHeight: 400 }}>
          <Table stickyHeader size="small">
            <TableHead>
              <TableRow>
                <TableCell width="50">#</TableCell>
                <TableCell>节点名称</TableCell>
                <TableCell width="100">延迟</TableCell>
                <TableCell width="80">状态</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {sortedResults.map((item, index) => (
                <TableRow key={item.name} hover>
                  <TableCell>{index + 1}</TableCell>
                  <TableCell>
                    <Typography variant="body2" noWrap>
                      {item.name}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip
                      label={formatDelay(item.delay)}
                      size="small"
                      color={getDelayColor(item.delay)}
                    />
                  </TableCell>
                  <TableCell>
                    {item.delay !== null ? (
                      <CheckCircle fontSize="small" color="success" />
                    ) : (
                      <Error fontSize="small" color="error" />
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>关闭</Button>
      </DialogActions>
    </Dialog>
  );
};

export default SpeedTestResultDialog;
