import React, { useState, useEffect, useCallback, useRef } from "react";
import {
  Card,
  CardContent,
  Typography,
  Box,
  Switch,
  Divider,
  Chip,
  Alert,
  Button,
  CircularProgress,
  IconButton,
  Tooltip,
  Skeleton,
} from "@mui/material";
import { Router, Settings, AdminPanelSettings, Refresh, HubOutlined } from "@mui/icons-material";
import { invoke } from "@tauri-apps/api/tauri";
import { useTranslation } from "react-i18next";
import { isTauriEnv } from "../utils/tauri";
import { useAppStore } from "../store/appStore";
import { catalogNodes } from '../utils/proxyCatalog';
import type { ConfigValue, ProxiesResponse, ProxyNode } from "../types";

interface SystemStatusCardProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: "success" | "error" | "info" | "warning") => void;
}

const SystemStatusCard: React.FC<SystemStatusCardProps> = React.memo(
  ({ isRunning, showNotification }) => {
    const { t, i18n } = useTranslation();
    const zh = (i18n.resolvedLanguage || i18n.language).startsWith("zh");
    const labels = zh
      ? {
          description: "连接设置与节点概览",
          tunDescription: "通过虚拟网卡接管流量，具体出口由代理规则决定。",
          adminHint: "启用 TUN 需要管理员权限。",
          savedSetting: "已保存设置",
          notConfigured: "未配置",
          requiresRestart: "配置已保存；启动服务后应用。",
          applied: "配置已保存，服务重启完成。",
          configFailed: "无法读取配置，请重试。",
          statsFailed: "节点状态暂不可用，请刷新重试。",
          online: "在线节点",
          available: "节点总数",
          statsHint: "状态基于最近一次核心查询",
        }
      : {
          description: "Connection settings and node overview",
          tunDescription:
            "Route traffic through a virtual adapter. Proxy rules determine its egress.",
          adminHint: "Administrator privileges are required to enable TUN.",
          savedSetting: "Saved setting",
          notConfigured: "Not configured",
          requiresRestart: "Configuration saved. Start the service to apply it.",
          applied: "Configuration saved and service restarted.",
          configFailed: "Unable to read configuration. Please retry.",
          statsFailed: "Node status is unavailable. Refresh to retry.",
          online: "Online nodes",
          available: "Total nodes",
          statsHint: "Based on the latest core response",
        };
    const {
      isAdmin,
      adminRestartPending,
      setAdminRestartPending,
      lastConfigChange,
      lastProxyChange,
    } = useAppStore();
    const [tunMode, setTunMode] = useState(false);
    const [config, setConfig] = useState<ConfigValue | null>(null);
    const [stats, setStats] = useState<{ total: number; online: number } | null>(null);
    const [tunLoading, setTunLoading] = useState(false);
    const [loading, setLoading] = useState(false);
    const [configError, setConfigError] = useState(false);
    const [statsError, setStatsError] = useState(false);
    const requestId = useRef(0);
    const tunPending = useRef(false);

    const loadSystemStatus = useCallback(async () => {
      if (!isTauriEnv()) return;
      const currentRequest = ++requestId.current;
      setLoading(true);
      setConfigError(false);
      setStatsError(false);
      const [configResult, proxyResult] = await Promise.allSettled([
        invoke<ConfigValue>("get_mihomo_config"),
        isRunning ? invoke<ProxiesResponse>("get_proxies") : Promise.resolve(null),
      ]);
      if (currentRequest !== requestId.current) return;
      if (configResult.status === "fulfilled") {
        setConfig(configResult.value);
        if (!tunPending.current) setTunMode(configResult.value.tun?.enable ?? false);
      } else {
        setConfigError(true);
      }
      if (proxyResult.status === "fulfilled" && proxyResult.value) {
        const nodes = catalogNodes(proxyResult.value);
        setStats({
          total: nodes.length,
          online: nodes.filter(({ node }) => (node as ProxyNode).alive === true).length,
        });
      } else {
        setStats(null);
        setStatsError(isRunning);
      }
      setLoading(false);
    }, [isRunning]);

    useEffect(() => {
      void loadSystemStatus();
      return () => {
        requestId.current += 1;
      };
    }, [loadSystemStatus, lastConfigChange, lastProxyChange]);

    const handleRestartAsAdmin = async () => {
      if (!isTauriEnv()) return;
      try {
        setAdminRestartPending(true);
        await invoke("restart_as_admin");
        window.setTimeout(() => {
          setAdminRestartPending(false);
          showNotification(t("notifications.adminRestartTimeout"), "warning");
        }, 20000);
      } catch (error) {
        setAdminRestartPending(false);
        showNotification(`${t("systemStatus.restartAsAdminFailed")}: ${error}`, "error");
      }
    };

    const handleTunToggle = async (enable: boolean) => {
      if (tunPending.current || !isTauriEnv()) return;
      if (enable && !isAdmin) {
        showNotification(t("systemStatus.tunRequiresAdmin"), "warning");
        return;
      }
      tunPending.current = true;
      setTunLoading(true);
      try {
        await invoke<string>("enable_tun_mode", { enable });
        setTunMode(enable);
        if (isRunning) {
          try {
            await invoke<string>("restart_mihomo_service_cmd");
            showNotification(labels.applied, "success");
          } catch (error) {
            showNotification(`${t("systemStatus.tunSavedRestartFailed")}: ${error}`, "warning");
          }
        } else {
          showNotification(labels.requiresRestart, "info");
        }
      } catch (error) {
        showNotification(`${t("systemStatus.tunToggleFailed")}: ${error}`, "error");
      } finally {
        tunPending.current = false;
        setTunLoading(false);
        void loadSystemStatus();
      }
    };

    const hasTunSetting = !configError && typeof config?.tun?.enable === "boolean";
    const settings = [
      { label: t("systemStatus.httpPort"), value: config?.port },
      { label: t("systemStatus.socksPort"), value: config?.["socks-port"] },
      { label: t("systemStatus.mixedPort"), value: config?.["mixed-port"] },
      { label: t("systemStatus.mode"), value: config?.mode },
      {
        label: t("systemStatus.dnsStatus"),
        value:
          typeof config?.dns?.enable === "boolean"
            ? config.dns.enable
              ? t("common.enabled")
              : t("common.disabled")
            : null,
      },
    ];

    return (
      <Card>
        <CardContent sx={{ p: { xs: 2.5, lg: 3 }, "&:last-child": { pb: { xs: 2.5, lg: 3 } } }}>
          <Box
            sx={{
              display: "flex",
              alignItems: "flex-start",
              justifyContent: "space-between",
              gap: 2,
              mb: 2.5,
            }}
          >
            <Box>
              <Typography variant="h6">{t("systemStatus.title")}</Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
                {labels.description}
              </Typography>
            </Box>
            <Tooltip title={t("common.refresh")}>
              <span>
                <IconButton
                  aria-label={t("common.refresh")}
                  size="small"
                  disabled={loading || !isTauriEnv()}
                  onClick={() => void loadSystemStatus()}
                  sx={{ border: "1px solid", borderColor: "divider" }}
                >
                  {loading ? <CircularProgress size={18} /> : <Refresh fontSize="small" />}
                </IconButton>
              </span>
            </Tooltip>
          </Box>
          {configError && (
            <Alert severity="warning" sx={{ mb: 2 }}>
              {labels.configFailed}
            </Alert>
          )}
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: {
                xs: "repeat(2, minmax(0, 1fr))",
                sm: "repeat(3, minmax(0, 1fr))",
              },
              gap: 1.25,
            }}
          >
            {settings.map(({ label, value }) => (
              <Box
                key={label}
                sx={{ p: 1.5, borderRadius: 1.5, bgcolor: "action.hover", minWidth: 0 }}
              >
                <Typography
                  variant="caption"
                  color="text.secondary"
                  display="block"
                  sx={{ mb: 0.5 }}
                >
                  {label}
                </Typography>
                {loading && !config ? (
                  <Skeleton width="55%" />
                ) : (
                  <Typography
                    variant="subtitle2"
                    sx={{
                      fontVariantNumeric: "tabular-nums",
                      overflowWrap: "anywhere",
                      userSelect: "text",
                    }}
                  >
                    {value ?? "—"}
                  </Typography>
                )}
              </Box>
            ))}
          </Box>
          <Divider sx={{ my: 2.5 }} />
          <Box sx={{ display: "flex", alignItems: "flex-start", gap: 1.5 }}>
            <Box
              sx={{
                display: "flex",
                p: 1,
                borderRadius: 1.5,
                bgcolor: "action.hover",
                color: "primary.main",
              }}
            >
              <Router fontSize="small" />
            </Box>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Box sx={{ display: "flex", alignItems: "center", gap: 1, flexWrap: "wrap" }}>
                <Typography variant="subtitle2">{t("systemStatus.tunMode")}</Typography>
                {hasTunSetting && (
                  <Chip size="small" variant="outlined" label={labels.savedSetting} />
                )}
                {tunLoading && <CircularProgress size={14} aria-label={t("common.loading")} />}
              </Box>
              <Typography variant="body2" color="text.secondary" sx={{ mt: 0.75, maxWidth: 460 }}>
                {labels.tunDescription}
              </Typography>
            </Box>
            {hasTunSetting ? (
              <Switch
                checked={tunMode}
                onChange={(event) => void handleTunToggle(event.target.checked)}
                disabled={tunLoading || !isTauriEnv()}
                inputProps={{ "aria-label": t("systemStatus.tunMode") }}
                sx={{ mt: -0.75, mr: -1 }}
              />
            ) : (
              <Box
                sx={{
                  display: "flex",
                  alignItems: "center",
                  gap: 1,
                  flexWrap: "wrap",
                  justifyContent: "flex-end",
                }}
              >
                <Chip
                  size="small"
                  variant="outlined"
                  label={
                    loading
                      ? t("common.loading")
                      : config && !configError
                        ? labels.notConfigured
                        : t("common.unknown")
                  }
                />
                {config && !configError && (
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={loading || tunLoading || !isTauriEnv()}
                    onClick={() => void handleTunToggle(true)}
                  >
                    {t("systemStatus.enableTun")}
                  </Button>
                )}
              </Box>
            )}
          </Box>
          {isAdmin === false && (
            <Alert severity="info" sx={{ mt: 2, "& .MuiAlert-message": { width: "100%" } }}>
              <Typography variant="body2" sx={{ mb: 1 }}>
                {labels.adminHint}
              </Typography>
              <Button
                color="inherit"
                size="small"
                variant="outlined"
                onClick={() => void handleRestartAsAdmin()}
                disabled={adminRestartPending || !isTauriEnv()}
                startIcon={<AdminPanelSettings fontSize="small" />}
              >
                {adminRestartPending
                  ? t("permissions.restartingAsAdmin")
                  : t("permissions.restartAsAdmin")}
              </Button>
            </Alert>
          )}
          {isRunning && (
            <>
              <Divider sx={{ my: 2.5 }} />
              <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 1.5 }}>
                <HubOutlined sx={{ fontSize: 18, color: "primary.main" }} />
                <Typography variant="subtitle2">{t("systemStatus.proxyCount")}</Typography>
              </Box>
              {statsError ? (
                <Alert severity="warning">{labels.statsFailed}</Alert>
              ) : (
                <Box
                  sx={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 2 }}
                >
                  {[
                    { label: labels.available, value: stats?.total, color: "text.primary" },
                    { label: labels.online, value: stats?.online, color: "success.main" },
                  ].map((metric) => (
                    <Box key={metric.label}>
                      <Typography
                        variant="h4"
                        sx={{
                          fontSize: "1.65rem",
                          fontWeight: 650,
                          color: metric.color,
                          fontVariantNumeric: "tabular-nums",
                        }}
                      >
                        {metric.value ?? "—"}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        {metric.label}
                      </Typography>
                    </Box>
                  ))}
                </Box>
              )}
              <Typography
                variant="caption"
                color="text.disabled"
                sx={{ mt: 1.5, display: "block" }}
              >
                {labels.statsHint}
              </Typography>
            </>
          )}
          {!isRunning && config && !configError && (
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 0.75,
                mt: 2,
                color: "text.secondary",
              }}
            >
              <Settings sx={{ fontSize: 15 }} />
              <Typography variant="caption">{labels.requiresRestart}</Typography>
            </Box>
          )}
        </CardContent>
      </Card>
    );
  }
);

SystemStatusCard.displayName = "SystemStatusCard";
export default SystemStatusCard;
