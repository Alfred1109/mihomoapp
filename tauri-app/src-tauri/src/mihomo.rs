use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    pub r#type: String,
    pub now: Option<String>,
    pub all: Vec<String>,
    pub history: Vec<ProxyHistory>,
}

pub async fn reload_config() -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo");
    let config_path = config_dir.join("config.yaml");

    let client = reqwest::Client::new();
    let response = client
        .put("http://127.0.0.1:9090/configs")
        .json(&serde_json::json!({ "path": config_path }))
        .send()
        .await
        .context("Failed to reload config")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to reload config: {}", response.status()));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHistory {
    pub name: String,
    pub delay: u32,
    pub time: String,
}

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
    std::fs::create_dir_all(std::path::Path::new(&config_path).parent().unwrap())
        .context("Failed to create config directory")?;
    
    if !std::path::Path::new(&config_path).exists() {
        crate::config::save_config(create_default_config()).await
            .context("Failed to create default config")?;
    }
    
    // 使用应用目录下的 mihomo.exe
    let app_dir = std::env::current_exe()
        .context("获取应用目录失败")?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("无法获取应用目录"))?
        .to_path_buf();
    
    let mihomo_path = app_dir.join("mihomo.exe");
    
    if !mihomo_path.exists() {
        return Err(anyhow::anyhow!("未找到 mihomo.exe 文件: {}", mihomo_path.display()));
    }
    
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
        "dns": {
            "enable": true,
            "listen": "0.0.0.0:53",
            "enhanced-mode": "fake-ip",
            "nameserver": [
                "https://doh.pub/dns-query",
                "https://dns.alidns.com/dns-query"
            ]
        },
        "proxies": [],
        "proxy-groups": [],
        "rules": [
            "DOMAIN-SUFFIX,local,DIRECT",
            "IP-CIDR,127.0.0.0/8,DIRECT",
            "IP-CIDR,172.16.0.0/12,DIRECT", 
            "IP-CIDR,192.168.0.0/16,DIRECT",
            "IP-CIDR,10.0.0.0/8,DIRECT",
            "GEOIP,LAN,DIRECT",
            "GEOIP,CN,DIRECT",
            "MATCH,PROXY"
        ]
    })
}

pub async fn stop_mihomo() -> Result<()> {
    // Try to gracefully stop mihomo via API first
    if let Err(_) = send_shutdown_command().await {
        // If API shutdown fails, force kill the process
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

fn get_config_path() -> Result<String> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo");
    
    let config_path = config_dir.join("config.yaml");
    
    Ok(config_path.to_string_lossy().to_string())
}
