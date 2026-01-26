use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use tokio::io::AsyncReadExt;
use tracing::{info, debug};

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
    std::fs::create_dir_all(config_dir)
        .context("Failed to create config directory")?;
    
    if !std::path::Path::new(&config_path).exists() {
        crate::config::save_config(create_default_config()).await
            .context("Failed to create default config")?;
    }
    
    // 检查是否已有mihomo进程在运行，如果有则先清理
    if is_mihomo_running().await {
        info!("Detected existing mihomo process, stopping it first...");
        let _ = stop_mihomo().await; // 忽略错误，继续启动
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    // 查找 mihomo 可执行文件
    // 优先使用系统路径，然后才查找应用目录
    
    #[cfg(target_os = "windows")]
    let mihomo_binary = "mihomo.exe";
    #[cfg(not(target_os = "windows"))]
    let mihomo_binary = "mihomo";
    
    // 系统路径列表
    #[cfg(target_os = "windows")]
    let system_paths: Vec<&str> = vec![];
    #[cfg(not(target_os = "windows"))]
    let system_paths = vec![
        "/usr/local/bin/mihomo",
        "/usr/bin/mihomo",
        "/opt/mihomo/mihomo",
    ];
    
    // 首先检查系统路径
    let mut mihomo_path = None;
    for path in &system_paths {
        if std::path::Path::new(path).exists() {
            mihomo_path = Some(std::path::PathBuf::from(path));
            break;
        }
    }
    
    // 如果系统路径找不到，检查应用目录
    if mihomo_path.is_none() {
        let app_dir = std::env::current_exe()
            .context("获取应用目录失败")?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("无法获取应用目录"))?
            .to_path_buf();
        
        // 检查多个可能的位置
        let possible_paths = vec![
            app_dir.join(mihomo_binary),                    // 应用目录根目录
            app_dir.join("resources").join(mihomo_binary),  // resources 子目录（Tauri 打包位置）
        ];
        
        for path in possible_paths {
            if path.exists() {
                mihomo_path = Some(path);
                break;
            }
        }
    }
    
    let mihomo_path = mihomo_path.ok_or_else(|| {
        anyhow::anyhow!("未找到 {} 文件。请确保 mihomo 已安装在系统路径或应用目录中", mihomo_binary)
    })?;
    
    match TokioCommand::new(&mihomo_path)
        .args(["-f", &config_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
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
                        if let Ok(_) = stderr.read_to_string(&mut error_output).await {
                            return Err(anyhow::anyhow!("Mihomo process exited with status: {} - {}", status, error_output));
                        } else {
                            return Err(anyhow::anyhow!("Mihomo process exited with status: {}", status));
                        }
                    } else {
                        return Err(anyhow::anyhow!("Mihomo process exited with status: {}", status));
                    }
                }
                Ok(None) => {
                    let pid = cmd.id().unwrap_or(0);
                    return Ok(pid);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to check mihomo process status: {}", e));
                }
            }
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to start mihomo with path '{}': {}", mihomo_path.display(), e));
        }
    }
}

pub fn create_default_config() -> serde_json::Value {
    serde_json::json!({
        "port": 7890,
        "socks-port": 7891,
        "mixed-port": 7890,
        "allow-lan": false,
        "mode": "rule",
        "log-level": "info",
        "external-controller": "127.0.0.1:9090",
        "unified-delay": true,
        "tcp-concurrent": true,
        "keep-alive-interval": 30,
        "find-process-mode": "strict",
        "global-client-fingerprint": "chrome",
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
                "+.battle.net"
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
            "IP-CIDR,127.0.0.0/8,DIRECT",
            "IP-CIDR,172.16.0.0/12,DIRECT", 
            "IP-CIDR,192.168.0.0/16,DIRECT",
            "IP-CIDR,10.0.0.0/8,DIRECT",
            "IP-CIDR,17.0.0.0/8,DIRECT",
            "IP-CIDR,100.64.0.0/10,DIRECT",
            "GEOSITE,private,DIRECT",
            "GEOSITE,cn,DIRECT",
            "GEOSITE,category-ads-all,REJECT",
            "GEOSITE,apple-cn,DIRECT",
            "GEOSITE,microsoft@cn,DIRECT",
            "GEOSITE,steam@cn,DIRECT",
            "GEOSITE,category-games@cn,DIRECT",
            "GEOIP,LAN,DIRECT,no-resolve",
            "GEOIP,CN,DIRECT,no-resolve",
            "MATCH,DIRECT"
        ]
    })
}

pub async fn stop_mihomo() -> Result<()> {
    // Try to gracefully stop mihomo via API first
    if let Ok(_) = send_shutdown_command().await {
        info!("Sent shutdown command to mihomo, waiting for graceful shutdown...");
        
        // 等待最多5秒让进程优雅关闭
        for i in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if !is_mihomo_running().await {
                info!("Mihomo stopped gracefully");
                return Ok(());
            }
            if i == 4 {
                info!("Mihomo still running after 2.5s, continuing to wait...");
            }
        }
        
        // 如果5秒后还在运行，强制杀进程
        warn!("Mihomo did not stop gracefully, force killing...");
        kill_mihomo_process().await?;
    } else {
        // API关闭失败，直接强制杀进程
        info!("API shutdown failed, force killing mihomo process...");
        kill_mihomo_process().await?;
    }
    
    Ok(())
}

async fn send_shutdown_command() -> Result<()> {
    let client = reqwest::Client::new();
    client
        .delete("http://127.0.0.1:9090/configs")
        .send()
        .await
        .context("Failed to send shutdown command")?;
    Ok(())
}

async fn kill_mihomo_process() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(["/F", "/IM", "mihomo.exe"])
            .output()
            .context("Failed to kill mihomo process")?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("pkill")
            .arg("mihomo")
            .output()
            .context("Failed to kill mihomo process")?;
    }
    
    Ok(())
}

/// 检查mihomo是否正在运行
async fn is_mihomo_running() -> bool {
    // 尝试通过API检查
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();
    
    if let Ok(client) = client {
        if let Ok(response) = client.get("http://127.0.0.1:9090/version").send().await {
            if response.status().is_success() {
                return true;
            }
        }
    }
    
    false
}

pub async fn get_proxies() -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:9090/proxies")
        .send()
        .await
        .context("Failed to fetch proxies")?;
    
    let proxies: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse proxies response")?;
    
    Ok(proxies)
}

pub async fn switch_proxy(group_name: &str, proxy_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let mut body = std::collections::HashMap::new();
    body.insert("name", proxy_name);
    
    let response = client
        .put(&format!("http://127.0.0.1:9090/proxies/{}", group_name))
        .json(&body)
        .send()
        .await
        .context("Failed to switch proxy")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to switch proxy: {}", response.status()));
    }
    
    Ok(())
}

#[allow(dead_code)]
pub async fn get_traffic_stats() -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:9090/traffic")
        .send()
        .await
        .context("Failed to fetch traffic stats")?;
    
    let stats: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse traffic stats")?;
    
    Ok(stats)
}

#[allow(dead_code)]
pub async fn test_proxy_delay(proxy_name: &str, test_url: &str, timeout: u32) -> Result<u32> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://127.0.0.1:9090/proxies/{}/delay", proxy_name))
        .query(&[("timeout", timeout.to_string()), ("url", test_url.to_string())])
        .send()
        .await
        .context("Failed to test proxy delay")?;
    
    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse delay test result")?;
    
    result["delay"]
        .as_u64()
        .map(|d| d as u32)
        .ok_or_else(|| anyhow::anyhow!("Invalid delay response"))
}

pub async fn test_group_delay(group_name: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("http://127.0.0.1:9090/group/{}/delay", group_name))
        .query(&[("timeout", "5000"), ("url", "http://www.gstatic.com/generate_204")])
        .send()
        .await
        .context("Failed to test group delay")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to test group delay: {}", response.status()));
    }
    
    Ok(())
}

/// 批量测试所有代理组的延迟（会更新Mihomo内部状态）
pub async fn test_all_groups_delay() -> Result<serde_json::Value> {
    info!("🚀 开始批量测试所有代理组延迟");
    
    // 获取所有代理信息
    let proxies = get_proxies().await?;
    let proxy_map = proxies["proxies"].as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid proxies response"))?;
    
    // 找出所有代理组（Selector, URLTest, Fallback等）
    let group_types = vec!["Selector", "URLTest", "Fallback", "LoadBalance"];
    let mut groups = Vec::new();
    
    for (name, proxy) in proxy_map {
        if let Some(proxy_type) = proxy["type"].as_str() {
            if group_types.contains(&proxy_type) {
                groups.push(name.clone());
            }
        }
    }
    
    info!("📊 找到 {} 个代理组", groups.len());
    
    let mut success_count = 0;
    let mut failed_groups = Vec::new();
    
    // 对每个组进行测速
    for group_name in &groups {
        info!("测试代理组: {}", group_name);
        match test_group_delay(group_name).await {
            Ok(_) => {
                success_count += 1;
                info!("  ✓ {} 测速完成", group_name);
            }
            Err(e) => {
                failed_groups.push(group_name.clone());
                info!("  ✗ {} 测速失败: {}", group_name, e);
            }
        }
    }
    
    info!("✅ 批量测速完成！成功: {}/{}", success_count, groups.len());
    
    Ok(serde_json::json!({
        "total": groups.len(),
        "success": success_count,
        "failed": failed_groups.len(),
        "failed_groups": failed_groups
    }))
}

/// 批量测试所有节点延迟（优化版）
pub async fn test_all_proxies_delay(test_url: Option<String>, timeout: Option<u32>) -> Result<serde_json::Value> {
    // 使用更快的测速URL - CP.cloudflare.com是专门用于连接测试的
    let test_url = test_url.unwrap_or_else(|| "http://cp.cloudflare.com".to_string());
    let timeout = timeout.unwrap_or(5000);
    
    info!("🚀 开始批量测速，测试URL: {}, 超时: {}ms", test_url, timeout);
    
    // 创建优化的HTTP客户端（连接池复用）
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .pool_idle_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_millis(timeout as u64))
        .tcp_keepalive(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;
    
    // 获取所有代理节点
    let proxies = get_proxies().await?;
    let proxy_map = proxies["proxies"].as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid proxies response"))?;
    
    // 过滤出实际的代理节点
    let exclude_types = vec!["Selector", "URLTest", "Fallback", "LoadBalance", "Relay"];
    let exclude_names = vec!["DIRECT", "REJECT", "COMPATIBLE", "PASS", "REJECT-DROP", "GLOBAL"];
    
    let mut proxy_names = Vec::new();
    for (name, proxy) in proxy_map {
        if let Some(proxy_type) = proxy["type"].as_str() {
            if !exclude_types.contains(&proxy_type) && !exclude_names.contains(&name.as_str()) {
                proxy_names.push(name.clone());
            }
        }
    }
    
    info!("📊 找到 {} 个节点，开始并发测速...", proxy_names.len());
    
    // 使用信号量限制并发数
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(20));
    let client = std::sync::Arc::new(client);
    
    let mut test_tasks = Vec::new();
    
    for name in proxy_names.iter() {
        let name_clone = name.clone();
        let url_clone = test_url.clone();
        let client_clone = client.clone();
        let semaphore_clone = semaphore.clone();
        
        let task = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.ok();
            
            match test_proxy_delay_optimized(&client_clone, &name_clone, &url_clone, timeout).await {
                Ok(delay) => {
                    debug!("  ✓ {} - {}ms", name_clone, delay);
                    (name_clone, Some(delay))
                }
                Err(_) => {
                    debug!("  ✗ {} - 超时", name_clone);
                    (name_clone, None)
                }
            }
        });
        
        test_tasks.push(task);
    }
    
    // 等待所有测试完成
    let mut results = std::collections::HashMap::new();
    for task in test_tasks {
        if let Ok((name, delay)) = task.await {
            results.insert(name, delay);
        }
    }
    
    let success_count = results.iter().filter(|(_, d)| d.is_some()).count();
    info!("✅ 批量测速完成！成功: {}/{}", success_count, proxy_names.len());
    
    Ok(serde_json::json!({
        "total": proxy_names.len(),
        "tested": results.len(),
        "success": success_count,
        "results": results
    }))
}

/// 优化的延迟测试（使用共享客户端）
async fn test_proxy_delay_optimized(
    client: &reqwest::Client,
    proxy_name: &str,
    test_url: &str,
    timeout: u32,
) -> Result<u32> {
    let response = client
        .get(&format!("http://127.0.0.1:9090/proxies/{}/delay", proxy_name))
        .query(&[("timeout", timeout.to_string()), ("url", test_url.to_string())])
        .send()
        .await
        .context("Failed to test proxy delay")?;
    
    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse delay test result")?;
    
    if let Some(delay) = result["delay"].as_u64() {
        Ok(delay as u32)
    } else {
        Err(anyhow::anyhow!("No delay value in response"))
    }
}

fn get_config_path() -> Result<String> {
    #[cfg(target_os = "windows")]
    let config_dir = {
        // Windows: 使用 AppData/Roaming 目录
        dirs::config_dir()
            .context("Failed to get config directory")?
            .join("mihomo")
    };
    
    #[cfg(not(target_os = "windows"))]
    let config_dir = {
        // Linux/Unix: 优先使用 SUDO_USER 环境变量获取实际用户的 home 目录
        // 这样即使以 root 运行，也会使用实际用户的配置
        if let Ok(sudo_user) = std::env::var("SUDO_USER") {
            // 如果是通过 sudo 运行，使用 sudo 用户的配置目录
            let user_home = std::path::PathBuf::from(format!("/home/{}", sudo_user));
            user_home.join(".config").join("mihomo")
        } else if let Ok(user) = std::env::var("USER") {
            // 如果是普通用户，使用其配置目录
            if user != "root" {
                let user_home = std::path::PathBuf::from(format!("/home/{}", user));
                user_home.join(".config").join("mihomo")
            } else {
                // root 用户使用默认配置目录
                dirs::config_dir()
                    .context("Failed to get config directory")?
                    .join("mihomo")
            }
        } else {
            // 后备方案：使用 dirs 库获取配置目录
            dirs::config_dir()
                .context("Failed to get config directory")?
                .join("mihomo")
        }
    };
    
    let config_path = config_dir.join("config.yaml");
    
    Ok(config_path.to_string_lossy().to_string())
}
