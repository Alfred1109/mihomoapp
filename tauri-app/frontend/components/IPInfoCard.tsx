import React, { useState, useEffect, useCallback, useRef } from "react";
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
  Skeleton,
  Button,
} from "@mui/material";
import {
  Refresh,
  Language,
  LocationOn,
  Business,
  Public,
  ContentCopy,
  TravelExplore,
} from "@mui/icons-material";
import { invoke } from "@tauri-apps/api/tauri";
import { useTranslation } from "react-i18next";
import { isTauriEnv } from "../utils/tauri";
import type { IPInfo } from "../types";

interface ExtendedIPInfo extends IPInfo {
  proxy_status?: "proxied" | "direct";
  country_code?: string;
  stale?: boolean;
}

interface IPInfoCardProps {
  isRunning: boolean;
  showNotification: (message: string, severity?: "success" | "error" | "info" | "warning") => void;
}

function readCachedIp(): ExtendedIPInfo | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem("ipInfo") || "null");
    if (
      value &&
      typeof value === "object" &&
      (typeof (value as IPInfo).ip === "string" || typeof (value as IPInfo).query === "string")
    ) {
      return { ...(value as ExtendedIPInfo), stale: true };
    }
  } catch {
    /* An unavailable or damaged cache must not prevent a fresh request. */
  }
  return null;
}

const IPInfoCard: React.FC<IPInfoCardProps> = React.memo(({ isRunning, showNotification }) => {
  const { t, i18n } = useTranslation();
  const zh = (i18n.resolvedLanguage || i18n.language).startsWith("zh");
  const labels = zh
    ? {
        description: "查看当前网络出口与归属信息",
        region: "地区",
        cached: "缓存结果",
        cachedHint: "上次查询结果，仅供参考。刷新后可确认当前出口。",
        proxied: "代理出口",
        direct: "直连出口",
        routeHint: "出口状态来自本次 IP 查询；具体应用的路由由代理规则决定。",
        copy: "复制 IP 地址",
        copied: "IP 地址已复制",
        copyFailed: "复制失败，请手动选择地址复制",
        empty: "尚未获取出口信息",
        retry: "重新检测",
      }
    : {
        description: "Inspect your network egress and location",
        region: "Region",
        cached: "Cached result",
        cachedHint: "Showing a previous result. Refresh to check the current egress.",
        proxied: "Proxy egress",
        direct: "Direct egress",
        routeHint: "Egress reflects this IP lookup. Individual apps follow your routing rules.",
        copy: "Copy IP address",
        copied: "IP address copied",
        copyFailed: "Unable to copy. Select and copy the address manually.",
        empty: "No egress information yet",
        retry: "Check again",
      };
  const [ipInfo, setIpInfo] = useState<ExtendedIPInfo | null>(readCachedIp);
  const [ipLoading, setIpLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const requestId = useRef(0);
  const pending = useRef(false);

  const loadIpInfo = useCallback(async () => {
    if (!isTauriEnv() || pending.current) return;
    pending.current = true;
    const currentRequest = ++requestId.current;
    setIpLoading(true);
    setLoadError(null);
    try {
      const data = await invoke<ExtendedIPInfo>("get_current_ip");
      if (currentRequest !== requestId.current) return;
      setIpInfo(data);
      try {
        localStorage.setItem("ipInfo", JSON.stringify(data));
        localStorage.setItem("ipInfoTime", Date.now().toString());
      } catch {
        /* Live results remain useful when local storage is unavailable. */
      }
    } catch {
      if (currentRequest !== requestId.current) return;
      setIpInfo((previous) => (previous ? { ...previous, stale: true } : null));
      setLoadError(t("ipInfo.networkError"));
    } finally {
      if (currentRequest === requestId.current) {
        pending.current = false;
        setIpLoading(false);
      }
    }
  }, [t]);

  useEffect(() => {
    // Existing results stay visible while the route settles after a service change.
    requestId.current += 1;
    pending.current = false;
    setIpInfo((previous) => (previous ? { ...previous, stale: true } : null));
    const timer = window.setTimeout(() => void loadIpInfo(), isRunning ? 1500 : 0);
    return () => window.clearTimeout(timer);
  }, [isRunning, loadIpInfo]);

  useEffect(
    () => () => {
      requestId.current += 1;
      pending.current = false;
    },
    []
  );

  const address = ipInfo?.query || ipInfo?.ip;
  const copyAddress = async () => {
    if (!address) return;
    try {
      await navigator.clipboard.writeText(address);
      showNotification(labels.copied, "success");
    } catch {
      showNotification(labels.copyFailed, "warning");
    }
  };

  const countryCode = ipInfo?.countryCode || ipInfo?.country_code;
  const flag =
    countryCode && /^[a-z]{2}$/i.test(countryCode)
      ? String.fromCodePoint(
          ...countryCode
            .toUpperCase()
            .split("")
            .map((char) => 127397 + char.charCodeAt(0))
        )
      : null;
  const details = [
    {
      icon: <Language fontSize="small" />,
      label: t("ipInfo.country"),
      value: [flag, ipInfo?.country].filter(Boolean).join(" "),
    },
    {
      icon: <LocationOn fontSize="small" />,
      label: labels.region,
      value: ipInfo?.regionName || ipInfo?.region,
    },
    { icon: <LocationOn fontSize="small" />, label: t("ipInfo.city"), value: ipInfo?.city },
    {
      icon: <Business fontSize="small" />,
      label: t("ipInfo.isp"),
      value: ipInfo?.isp || ipInfo?.org,
    },
  ];

  return (
    <Card sx={{ height: "100%" }}>
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
            <Typography variant="h6">{t("ipInfo.title")}</Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
              {labels.description}
            </Typography>
          </Box>
          <Tooltip title={t("ipInfo.refresh")}>
            <span>
              <IconButton
                aria-label={t("ipInfo.refresh")}
                onClick={() => void loadIpInfo()}
                disabled={ipLoading || !isTauriEnv()}
                size="small"
                sx={{ border: "1px solid", borderColor: "divider" }}
              >
                {ipLoading ? <CircularProgress size={18} /> : <Refresh fontSize="small" />}
              </IconButton>
            </span>
          </Tooltip>
        </Box>

        {loadError && (
          <Alert
            severity="warning"
            sx={{ mb: 2 }}
            action={
              <Button
                color="inherit"
                size="small"
                disabled={ipLoading}
                onClick={() => void loadIpInfo()}
              >
                {t("common.retry")}
              </Button>
            }
          >
            {loadError}
          </Alert>
        )}

        {ipLoading && !ipInfo ? (
          <Box aria-label={t("ipInfo.loading")} sx={{ py: 1 }}>
            <Skeleton width="45%" height={20} />
            <Skeleton width="85%" height={44} sx={{ mb: 2 }} />
            {[0, 1, 2, 3].map((row) => (
              <Skeleton key={row} height={34} sx={{ my: 0.5 }} />
            ))}
          </Box>
        ) : ipInfo ? (
          <>
            <Box
              sx={{
                p: 2,
                borderRadius: 2,
                border: "1px solid",
                borderColor: "divider",
                bgcolor: "action.hover",
                mb: 2.5,
              }}
            >
              <Box sx={{ display: "flex", alignItems: "center", gap: 0.75, mb: 1 }}>
                <Public sx={{ fontSize: 16, color: "primary.main" }} />
                <Typography variant="caption" color="text.secondary">
                  {t("ipInfo.ip")}
                </Typography>
                {ipInfo.stale && (
                  <Tooltip title={labels.cachedHint}>
                    <Chip
                      label={labels.cached}
                      size="small"
                      variant="outlined"
                      sx={{ ml: "auto" }}
                    />
                  </Tooltip>
                )}
              </Box>
              <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                <Typography
                  sx={{
                    fontFamily: "monospace",
                    fontSize: { xs: "1.2rem", lg: "1.35rem" },
                    fontWeight: 600,
                    overflowWrap: "anywhere",
                    userSelect: "text",
                    flex: 1,
                    minWidth: 0,
                  }}
                >
                  {address || t("ipInfo.unknown")}
                </Typography>
                {address && (
                  <Tooltip title={labels.copy}>
                    <IconButton
                      size="small"
                      aria-label={labels.copy}
                      onClick={() => void copyAddress()}
                    >
                      <ContentCopy sx={{ fontSize: 17 }} />
                    </IconButton>
                  </Tooltip>
                )}
              </Box>
            </Box>
            <Box component="dl" sx={{ m: 0, display: "grid", gap: 1.75 }}>
              {details.map(({ icon, label, value }) => (
                <Box
                  key={label}
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "20px minmax(62px, auto) 1fr",
                    alignItems: "start",
                    gap: 1,
                  }}
                >
                  <Box sx={{ color: "text.disabled", display: "flex", pt: 0.1 }}>{icon}</Box>
                  <Typography component="dt" variant="body2" color="text.secondary">
                    {label}
                  </Typography>
                  <Typography
                    component="dd"
                    variant="body2"
                    sx={{ m: 0, textAlign: "right", overflowWrap: "anywhere", userSelect: "text" }}
                  >
                    {value || t("ipInfo.unknown")}
                  </Typography>
                </Box>
              ))}
            </Box>
            <Divider sx={{ my: 2.5 }} />
            <Box sx={{ display: "flex", gap: 1, flexWrap: "wrap" }}>
              <Chip
                label={isRunning ? t("dashboard.running") : t("dashboard.stopped")}
                color={isRunning ? "success" : "default"}
                size="small"
                variant="outlined"
              />
              {ipInfo.proxy_status && (
                <Tooltip title={labels.routeHint}>
                  <Chip
                    label={ipInfo.proxy_status === "proxied" ? labels.proxied : labels.direct}
                    color={ipInfo.proxy_status === "proxied" ? "primary" : "default"}
                    size="small"
                  />
                </Tooltip>
              )}
            </Box>
          </>
        ) : (
          <Box sx={{ textAlign: "center", py: 4 }}>
            <TravelExplore sx={{ fontSize: 44, color: "text.disabled", mb: 1.5 }} />
            <Typography variant="subtitle2" sx={{ mb: 1.5 }}>
              {labels.empty}
            </Typography>
            <Button
              variant="outlined"
              size="small"
              startIcon={<Refresh />}
              disabled={!isTauriEnv()}
              onClick={() => void loadIpInfo()}
            >
              {labels.retry}
            </Button>
          </Box>
        )}
      </CardContent>
    </Card>
  );
});

IPInfoCard.displayName = "IPInfoCard";
export default IPInfoCard;
