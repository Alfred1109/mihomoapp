use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tracing::info;

static OWNED_PROCESS: tokio::sync::Mutex<Option<tokio::process::Child>> =
    tokio::sync::Mutex::const_new(None);

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    pub r#type: String,
    pub now: Option<String>,
    pub all: Vec<String>,
    pub history: Vec<ProxyHistory>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHistory {
    pub name: String,
    pub delay: u32,
    pub time: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    pub r#type: String,
    pub delay: Option<u32>,
    pub alive: bool,
}

pub async fn start_mihomo() -> Result<u32> {
    let config_path = get_config_path()?;

    // Ensure config directory and file exist
    let config_dir = std::path::Path::new(&config_path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get config directory path"))?;
    std::fs::create_dir_all(config_dir).context("Failed to create config directory")?;

    if !std::path::Path::new(&config_path).exists() {
        crate::config::save_config(create_default_config())
            .await
            .context("Failed to create default config")?;
    }

    // Never stop or adopt an unrelated controller listening on this port.
    let mut owned_process = OWNED_PROCESS.lock().await;
    if let Some(child) = owned_process.as_mut() {
        if child.try_wait().context("检查受管理进程失败")?.is_none() {
            return Err(anyhow::anyhow!("本应用启动的 Mihomo 已在运行"));
        }
        *owned_process = None;
    }
    if is_mihomo_running().await {
        return Err(anyhow::anyhow!(
            "控制器端口已有 Mihomo 实例运行，请先确认其归属"
        ));
    }

    let mihomo_path = crate::core_binary::resolve_mihomo()?;
    crate::config::prepare_config_for_core(&mihomo_path).await?;

    // 在 Windows 上创建隐藏控制台窗口的命令
    #[cfg(target_os = "windows")]
    let mut cmd = {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut command = TokioCommand::new(&mihomo_path);
        command
            .args(["-f", &config_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        command
    };

    // 在非 Windows 系统上正常启动
    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut command = TokioCommand::new(&mihomo_path);
        command
            .args(["-f", &config_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    };

    match cmd.spawn() {
        Ok(mut cmd) => {
            // Give it a moment to start
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Check if the process is still running
            match cmd.try_wait() {
                Ok(Some(status)) => {
                    // Process exited, capture error output
                    let stderr = cmd.stderr.take();
                    if let Some(mut stderr) = stderr {
                        let mut error_output = String::new();
                        if (stderr.read_to_string(&mut error_output).await).is_ok() {
                            Err(anyhow::anyhow!(
                                "Mihomo process exited with status: {} - {}",
                                status,
                                error_output
                            ))
                        } else {
                            Err(anyhow::anyhow!(
                                "Mihomo process exited with status: {}",
                                status
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Mihomo process exited with status: {}",
                            status
                        ))
                    }
                }
                Ok(None) => {
                    let pid = cmd
                        .id()
                        .ok_or_else(|| anyhow::anyhow!("无法获取 Mihomo 进程 ID"))?;
                    // Drain both pipes after startup so sustained logging cannot block the child.
                    if let Some(mut stdout) = cmd.stdout.take() {
                        tokio::spawn(async move {
                            let _ = tokio::io::copy(&mut stdout, &mut tokio::io::sink()).await;
                        });
                    }
                    if let Some(mut stderr) = cmd.stderr.take() {
                        tokio::spawn(async move {
                            let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
                        });
                    }
                    *owned_process = Some(cmd);
                    crate::controller::clear_connection().await;
                    // The process handle establishes ownership; API validation is separate.
                    let _ = crate::controller::connect_configured().await;
                    Ok(pid)
                }
                Err(e) => Err(anyhow::anyhow!(
                    "Failed to check mihomo process status: {}",
                    e
                )),
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to start mihomo with path '{}': {}",
            mihomo_path.display(),
            e
        )),
    }
}

pub fn create_default_config() -> serde_json::Value {
    serde_json::json!({
        "port": 7890,
        "socks-port": 7891,
        "mixed-port": 7892,
        "allow-lan": false,
        "mode": "rule",
        "log-level": "info",
        "external-controller": "127.0.0.1:9090",
        "unified-delay": true,
        "tcp-concurrent": true,
        "keep-alive-interval": 30,
        "find-process-mode": "strict",
        "mihomo-manager-default-client-fingerprint": "chrome",
        "dns": {
            "enable": true,
            "ipv6": false,
            "listen": "0.0.0.0:53",
            "enhanced-mode": "fake-ip",
            "fake-ip-range": "198.18.0.1/16",
            "fake-ip-filter": [
                "*.lan",
                "*.local",
                "localhost.ptlogin2.qq.com",
                "+.srv.nintendo.net",
                "+.stun.playstation.net",
                "xbox.*.microsoft.com",
                "+.xboxlive.com",
                "+.battlenet.com.cn",
                "+.battlenet.com",
                "+.blzstatic.cn",
                "+.battle.net",
                "+.kuwo.cn",
                "+.msftconnecttest.com",
                "+.msftncsi.com"
            ],
            "default-nameserver": [
                "223.5.5.5",
                "119.29.29.29"
            ],
            "nameserver": [
                "https://doh.pub/dns-query",
                "https://dns.alidns.com/dns-query"
            ],
            "fallback": [
                "https://1.1.1.1/dns-query",
                "https://dns.google/dns-query"
            ],
            "prefer-h3": true,
            "fallback-filter": {
                "geoip": true,
                "geoip-code": "CN",
                "ipcidr": [
                    "240.0.0.0/4"
                ]
            },
            "nameserver-policy": {
                "geosite:cn,private,apple": [
                    "https://doh.pub/dns-query",
                    "https://dns.alidns.com/dns-query"
                ],
                "geosite:geolocation-!cn": [
                    "https://1.1.1.1/dns-query",
                    "https://dns.google/dns-query"
                ],
                "geosite:category-ads-all": "rcode://success"
            }
        },
        "tun": {
            "enable": false,
            "stack": "system",
            "auto-route": true,
            "auto-detect-interface": true,
            "dns-hijack": ["any:53"],
            "mtu": 1500
        },
        "proxies": [],
        "proxy-groups": [],
        "rules": [
            "DOMAIN-SUFFIX,local,DIRECT",
            "DOMAIN-SUFFIX,localhost,DIRECT",
            "IP-CIDR,127.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
            "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
            "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,17.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,100.64.0.0/10,DIRECT,no-resolve",
            "GEOIP,LAN,DIRECT,no-resolve",
            "GEOSITE,category-ads-all,ADBLOCK",
            "GEOSITE,private,DIRECT",
            "GEOSITE,cn,DIRECT",
            "GEOSITE,apple-cn,DIRECT",
            "GEOSITE,microsoft@cn,DIRECT",
            "GEOSITE,steam@cn,DIRECT",
            "GEOSITE,category-games@cn,DIRECT",
            "GEOSITE,geolocation-!cn,PROXY",
            "GEOIP,CN,DIRECT,no-resolve",
            "MATCH,PROXY"
        ]
    })
}

/// Only an exited child can be replaced automatically; an unavailable API
/// alone does not establish that the process has exited.
pub async fn owned_process_exited() -> Result<bool> {
    let mut owned_process = OWNED_PROCESS.lock().await;
    match owned_process.as_mut() {
        Some(child) => Ok(child.try_wait().context("检查受管理进程失败")?.is_some()),
        None => Ok(true),
    }
}

pub async fn stop_mihomo() -> Result<()> {
    let mut owned_process = OWNED_PROCESS.lock().await;
    if let Some(child) = owned_process.as_mut() {
        if child.try_wait().context("检查受管理进程失败")?.is_none() {
            child.kill().await.context("停止本应用启动的 Mihomo 失败")?;
        }
        *owned_process = None;
        crate::controller::clear_connection().await;
        info!("Stopped the Mihomo process started by this application");
    }
    Ok(())
}

/// Controller reachability is independent of process ownership.
pub async fn is_mihomo_running() -> bool {
    crate::controller::health().await.is_ok()
}

pub async fn get_proxies() -> Result<serde_json::Value> {
    let client =
        crate::controller::ControllerClient::active(std::time::Duration::from_secs(10)).await?;
    let mut response = client.get_json(&["proxies"]).await?;
    if !response["proxies"].is_object() {
        return Err(anyhow::anyhow!("Invalid proxies response"));
    }
    let provider_response = client.get_json(&["providers", "proxies"]).await?;
    let providers = provider_response
        .get("providers")
        .filter(|value| value.is_object())
        .context("Invalid providers response")?;
    response["providers"] = providers.clone();
    Ok(response)
}

pub async fn switch_proxy(group_name: &str, proxy_name: &str) -> Result<()> {
    let client =
        crate::controller::ControllerClient::active(std::time::Duration::from_secs(10)).await?;
    client
        .send(
            client
                .request(reqwest::Method::PUT, &["proxies", group_name])?
                .json(&serde_json::json!({"name": proxy_name})),
        )
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn get_traffic_stats() -> Result<serde_json::Value> {
    let client =
        crate::controller::ControllerClient::active(std::time::Duration::from_secs(10)).await?;
    let mut response = client
        .send(client.request(reqwest::Method::GET, &["traffic"])?)
        .await?;
    // /traffic is a JSON-lines stream. Read one sample instead of waiting for EOF.
    let mut sample = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| error.without_url())?
    {
        sample.extend_from_slice(&chunk);
        if let Some(end) = sample.iter().position(|byte| *byte == b'\n') {
            return serde_json::from_slice(&sample[..end]).context("核心流量采样无效");
        }
        if sample.len() > 65536 {
            return Err(anyhow::anyhow!("核心流量采样过大"));
        }
    }
    serde_json::from_slice(&sample).context("核心流量采样无效")
}

pub async fn test_group_delay(group_name: &str) -> Result<()> {
    let client =
        crate::controller::ControllerClient::active(std::time::Duration::from_secs(8)).await?;
    client
        .send(
            client
                .request(reqwest::Method::GET, &["group", group_name, "delay"])?
                .query(&[
                    ("timeout", "5000"),
                    ("url", "http://www.gstatic.com/generate_204"),
                ]),
        )
        .await?;
    Ok(())
}

/// Test a single node with a request deadline slightly above the core's deadline.
pub async fn test_proxy_delay(proxy_name: &str, timeout: u32, test_url: &str) -> Result<u32> {
    test_target_delay(
        &crate::proxy_catalog::ProxyTarget {
            name: proxy_name.to_owned(),
            provider: None,
        },
        timeout,
        test_url,
    )
    .await
}

async fn test_target_delay(
    target: &crate::proxy_catalog::ProxyTarget,
    timeout: u32,
    test_url: &str,
) -> Result<u32> {
    let client = crate::controller::ControllerClient::active(std::time::Duration::from_millis(
        u64::from(timeout) + 3000,
    ))
    .await?;
    let response = client
        .send(
            client
                .request(reqwest::Method::GET, &target.delay_path())?
                .query(&[("timeout", timeout.to_string().as_str()), ("url", test_url)]),
        )
        .await?;
    let result: serde_json::Value = response.json().await.map_err(|error| error.without_url())?;
    result["delay"]
        .as_u64()
        .and_then(|delay| u32::try_from(delay).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid delay response"))
}

/// Each provider/name pair has its own endpoint and result. Same-name nodes
/// from separate providers must never overwrite one another in a flat map.
pub async fn test_all_groups_delay() -> Result<serde_json::Value> {
    use futures::stream::{self, StreamExt};
    let targets = crate::proxy_catalog::test_targets(&get_proxies().await?)?;
    let total = targets.len();
    let tested: Vec<_> = stream::iter(targets)
        .map(|target| async move {
            let result = if target.provider.is_none() {
                test_proxy_delay(&target.name, 5000, "http://www.gstatic.com/generate_204").await
            } else {
                test_target_delay(&target, 5000, "http://www.gstatic.com/generate_204").await
            };
            (target, result)
        })
        .buffer_unordered(10)
        .collect()
        .await;
    let mut success = 0;
    let mut results = serde_json::Map::new();
    let mut provider_results = serde_json::Map::new();
    for (target, result) in tested {
        let value = match result {
            Ok(delay) => {
                success += 1;
                serde_json::json!(delay)
            }
            Err(error) => {
                info!("Node delay test failed for {}: {}", target.name, error);
                serde_json::Value::Null
            }
        };
        if let Some(provider) = target.provider {
            provider_results
                .entry(provider)
                .or_insert_with(|| serde_json::json!({}))[target.name] = value;
        } else {
            results.insert(target.name, value);
        }
    }
    Ok(
        serde_json::json!({"total": total, "tested": total, "success": success,
        "results": results, "provider_results": provider_results}),
    )
}

fn get_config_path() -> Result<String> {
    // 使用统一的平台配置系统
    let config_path = crate::platform_config::PlatformPaths::config_file()?;
    Ok(config_path.to_string_lossy().to_string())
}

#[cfg(all(test, unix))]
mod process_tests {
    use super::*;

    #[tokio::test]
    async fn batch_tests_provider_nodes_using_scoped_endpoints() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _test_guard = crate::controller::TEST_CONNECTION_LOCK.lock().await;
        crate::controller::clear_connection().await;
        let directory =
            std::env::temp_dir().join(format!("mihomo-provider-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config_path = directory.join("config.yaml");
        std::fs::write(
            &config_path,
            format!("external-controller: {}\n", listener.local_addr().unwrap()),
        )
        .unwrap();
        crate::config_manager::init_config_manager(config_path).await;
        let server = tokio::spawn(async move {
            let mut paths = Vec::new();
            for _ in 0..4 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = vec![0; 8192];
                let count = socket.read(&mut bytes).await.unwrap();
                let request = String::from_utf8_lossy(&bytes[..count]);
                let path = request.split_whitespace().nth(1).unwrap().to_owned();
                let body = if path == "/proxies" {
                    r#"{"proxies":{"same":{"type":"Vless"},"PROXY":{"type":"Selector"},"DIRECT":{"type":"Direct"}}}"#
                } else if path == "/providers/proxies" {
                    r#"{"providers":{"source/a":{"vehicleType":"HTTP","proxies":[{"name":"same","type":"Trojan"}]},"default":{"vehicleType":"Compatible","proxies":[{"name":"same","type":"Vless"}]}}}"#
                } else if path.starts_with("/providers/proxies/source%2Fa/same/healthcheck?") {
                    r#"{"delay":23}"#
                } else if path.starts_with("/proxies/same/delay?") {
                    r#"{"delay":17}"#
                } else {
                    panic!("Unexpected mock request: {}", path);
                };
                paths.push(path);
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
            paths
        });
        let result = test_all_groups_delay().await.unwrap();
        assert_eq!(result["total"], 2);
        assert_eq!(result["success"], 2);
        assert_eq!(result["results"]["same"], 17);
        assert_eq!(result["provider_results"]["source/a"]["same"], 23);
        assert_eq!(server.await.unwrap().len(), 4);
        crate::controller::clear_connection().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn stop_only_terminates_the_owned_child() {
        let _test_guard = crate::controller::TEST_CONNECTION_LOCK.lock().await;
        let owned = TokioCommand::new("sleep")
            .arg("30")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut unrelated = TokioCommand::new("sleep")
            .arg("30")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        *OWNED_PROCESS.lock().await = Some(owned);

        assert!(!owned_process_exited().await.unwrap());
        stop_mihomo().await.unwrap();
        assert!(owned_process_exited().await.unwrap());
        assert!(unrelated.try_wait().unwrap().is_none());
        unrelated.kill().await.unwrap();
    }
}
