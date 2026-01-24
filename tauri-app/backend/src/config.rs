use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use yaml_rust::Yaml;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MihomoConfig {
    pub port: Option<u32>,
    pub socks_port: Option<u32>,
    pub mixed_port: Option<u32>,
    pub allow_lan: Option<bool>,
    pub mode: Option<String>,
    pub log_level: Option<String>,
    pub external_controller: Option<String>,
    pub tun: Option<TunConfig>,
    pub dns: Option<DnsConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunConfig {
    pub enable: bool,
    pub stack: Option<String>,
    pub device_name: Option<String>,
    pub auto_route: Option<bool>,
    pub auto_detect_interface: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub enable: bool,
    pub listen: Option<String>,
    pub enhanced_mode: Option<String>,
    pub nameserver: Option<Vec<String>>,
    pub fallback: Option<Vec<String>>,
}

pub async fn load_config() -> Result<serde_json::Value> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        create_default_config(&config_path).await?;
    }
    
    let manager = crate::config_manager::get_config_manager().await?;
    manager.read_config().await
}

pub async fn save_config(config: serde_json::Value) -> Result<()> {
    let config_path = get_config_path()?;
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create config directory")?;
    }
    
    let manager = crate::config_manager::get_config_manager().await?;
    manager.write_config(config).await
}

pub async fn set_tun_mode(enable: bool) -> Result<()> {
    let mut config = load_config().await?;
    
    // Ensure tun section exists
    if config.get("tun").is_none() {
        config["tun"] = serde_json::json!({});
    }
    
    config["tun"]["enable"] = serde_json::json!(enable);
    
    // Set default TUN settings if enabling
    if enable {
        if config["tun"]["stack"].is_null() {
            config["tun"]["stack"] = serde_json::json!("system");
        }
        if config["tun"]["auto-route"].is_null() {
            config["tun"]["auto-route"] = serde_json::json!(true);
        }
        if config["tun"]["auto-detect-interface"].is_null() {
            config["tun"]["auto-detect-interface"] = serde_json::json!(true);
        }
        if config["tun"]["dns-hijack"].is_null() {
            config["tun"]["dns-hijack"] = serde_json::json!(["any:53"]);
        }
        if config["tun"]["mtu"].is_null() {
            config["tun"]["mtu"] = serde_json::json!(1500);
        }
    }
    
    save_config(config).await
}

async fn create_default_config(_config_path: &PathBuf) -> Result<()> {
    let default_config = serde_json::json!({
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
            "GEOSITE,geolocation-!cn,PROXY",
            "GEOIP,LAN,DIRECT,no-resolve",
            "GEOIP,CN,DIRECT,no-resolve",
            "MATCH,PROXY"
        ]
    });
    
    save_config(default_config).await
}

pub fn get_config_path() -> Result<PathBuf> {
    let config_dir = get_mihomo_config_dir()?;
    Ok(config_dir.join("config.yaml"))
}

fn get_mihomo_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 AppData/Roaming 目录
        Ok(dirs::config_dir()
            .context("Failed to get config directory")?
            .join("mihomo"))
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/Unix: 优先使用 SUDO_USER 环境变量获取实际用户的 home 目录
        // 这样即使以 root 运行，也会使用实际用户的配置
        let config_dir = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
            let user_home = PathBuf::from(format!("/home/{}", sudo_user));
            user_home.join(".config").join("mihomo")
        } else if let Ok(user) = std::env::var("USER") {
            if user != "root" {
                let user_home = PathBuf::from(format!("/home/{}", user));
                user_home.join(".config").join("mihomo")
            } else {
                dirs::config_dir()
                    .context("Failed to get config directory")?
                    .join("mihomo")
            }
        } else {
            dirs::config_dir()
                .context("Failed to get config directory")?
                .join("mihomo")
        };
        Ok(config_dir)
    }
}

pub fn yaml_to_json(yaml: &Yaml) -> Result<serde_json::Value> {
    match yaml {
        Yaml::Real(f) => Ok(serde_json::json!(f.parse::<f64>().unwrap_or(0.0))),
        Yaml::Integer(i) => Ok(serde_json::json!(*i)),
        Yaml::String(s) => Ok(serde_json::json!(s)),
        Yaml::Boolean(b) => Ok(serde_json::json!(*b)),
        Yaml::Array(arr) => {
            let mut json_arr = Vec::new();
            for item in arr {
                json_arr.push(yaml_to_json(item)?);
            }
            Ok(serde_json::json!(json_arr))
        }
        Yaml::Hash(hash) => {
            let mut json_obj = serde_json::Map::new();
            for (key, value) in hash {
                if let Yaml::String(key_str) = key {
                    json_obj.insert(key_str.clone(), yaml_to_json(value)?);
                }
            }
            Ok(serde_json::Value::Object(json_obj))
        }
        Yaml::Alias(_) => Err(anyhow::anyhow!("YAML aliases not supported")),
        Yaml::Null => Ok(serde_json::Value::Null),
        Yaml::BadValue => Err(anyhow::anyhow!("Bad YAML value")),
    }
}
