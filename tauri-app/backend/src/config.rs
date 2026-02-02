use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use yaml_rust::Yaml;

pub async fn load_config() -> Result<serde_json::Value> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        create_default_config(&config_path).await?;
    }

    let manager = crate::config_manager::get_config_manager().await?;
    let mut config = manager.read_config().await?;

    upgrade_config_if_needed(&mut config).await?;

    Ok(config)
}

pub async fn save_config(config: serde_json::Value) -> Result<()> {
    let config_path = get_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let manager = crate::config_manager::get_config_manager().await?;
    manager.write_config(config).await
}

#[allow(dead_code)]
pub async fn save_config_no_backup(config: serde_json::Value) -> Result<()> {
    let config_path = get_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }

    let manager = crate::config_manager::get_config_manager().await?;
    manager.write_config_with_options(config, false).await
}

pub async fn reset_to_default_config() -> Result<String> {
    tracing::info!("开始重置配置为默认值...");
    
    crate::base_config::reset_to_default().await?;
    
    let storage = crate::subscription::get_subscriptions().await.unwrap_or_default();
    let active_subscriptions: Vec<_> = storage.iter()
        .filter(|s| matches!(s.status, crate::subscription::SubscriptionStatus::Active) && s.proxy_count > 0)
        .collect();

    if active_subscriptions.is_empty() {
        let default_config = crate::base_config::create_default_base_config();
        let mut runtime_config = default_config.clone();
        runtime_config["proxies"] = serde_json::json!([]);
        runtime_config["proxy-groups"] = serde_json::json!([]);
        if let Some(obj) = runtime_config.as_object_mut() {
            obj.remove("base_config_version");
        }
        runtime_config["config_version"] = serde_json::json!(2);
        save_config(runtime_config).await?;
        
        tracing::info!("配置已重置为默认值（无活动订阅）");
        return Ok("配置已重置为默认值。请添加订阅链接或从备份恢复订阅。".to_string());
    }

    let subscription_ids: Vec<String> = active_subscriptions.iter()
        .map(|s| s.id.clone())
        .collect();
    
    match crate::subscription::generate_config_from_subscriptions(subscription_ids).await {
        Ok(_) => {
            tracing::info!("配置已重置为默认值并重新生成（使用现有订阅）");
            Ok("配置已重置为默认值，并使用现有订阅重新生成了运行时配置。".to_string())
        }
        Err(e) => {
            tracing::warn!("重新生成配置失败: {}", e);
            let default_config = crate::base_config::create_default_base_config();
            let mut runtime_config = default_config.clone();
            runtime_config["proxies"] = serde_json::json!([]);
            runtime_config["proxy-groups"] = serde_json::json!([]);
            if let Some(obj) = runtime_config.as_object_mut() {
                obj.remove("base_config_version");
            }
            runtime_config["config_version"] = serde_json::json!(2);
            save_config(runtime_config).await?;
            
            Ok(format!("配置已重置为默认值，但重新生成订阅配置失败: {}。请手动更新订阅。", e))
        }
    }
}

pub async fn set_tun_mode(enable: bool) -> Result<()> {
    let mut base_config = crate::base_config::load_base_config().await?;

    if base_config.get("tun").is_none() {
        base_config["tun"] = serde_json::json!({});
    }

    base_config["tun"]["enable"] = serde_json::json!(enable);

    if enable {
        if base_config["tun"]["stack"].is_null() {
            base_config["tun"]["stack"] = serde_json::json!("system");
        }
        if base_config["tun"]["auto-route"].is_null() {
            base_config["tun"]["auto-route"] = serde_json::json!(true);
        }
        if base_config["tun"]["auto-detect-interface"].is_null() {
            base_config["tun"]["auto-detect-interface"] = serde_json::json!(true);
        }
        if base_config["tun"]["dns-hijack"].is_null() {
            base_config["tun"]["dns-hijack"] = serde_json::json!(["any:53"]);
        }
        if base_config["tun"]["mtu"].is_null() {
            base_config["tun"]["mtu"] = serde_json::json!(1500);
        }
    }

    crate::base_config::save_base_config(&base_config).await?;

    let mut runtime_config = load_config().await?;
    runtime_config["tun"] = base_config["tun"].clone();
    save_config(runtime_config).await
}

const CONFIG_VERSION: u32 = 2;

async fn upgrade_config_if_needed(config: &mut serde_json::Value) -> Result<()> {
    let current_version = config
        .get("config_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    if current_version >= CONFIG_VERSION {
        return Ok(());
    }

    tracing::info!("检测到旧配置版本 {}，升级到版本 {}", current_version, CONFIG_VERSION);

    if current_version < 2 {
        upgrade_to_v2(config).await?;
    }

    save_config(config.clone()).await?;
    tracing::info!("配置已升级到版本 {}", CONFIG_VERSION);

    Ok(())
}

async fn upgrade_to_v2(config: &mut serde_json::Value) -> Result<()> {
    tracing::info!("应用 v2 性能优化...");

    config["config_version"] = serde_json::json!(2);

    if let Some(dns) = config.get_mut("dns") {
        dns["prefer-h3"] = serde_json::json!(true);

        if let Some(nameservers) = dns.get_mut("nameserver").and_then(|v| v.as_array_mut()) {
            nameservers.retain(|ns| {
                if let Some(s) = ns.as_str() {
                    !s.contains("[2400:3200")
                } else {
                    true
                }
            });
        }

        if let Some(fallback) = dns.get_mut("fallback").and_then(|v| v.as_array_mut()) {
            if fallback.len() > 2 {
                fallback.clear();
                fallback.push(serde_json::json!("https://1.1.1.1/dns-query"));
                fallback.push(serde_json::json!("https://dns.google/dns-query"));
            }
        }

        if let Some(policy) = dns.get_mut("nameserver-policy") {
            if let Some(geolocation) = policy.get_mut("geosite:geolocation-!cn") {
                *geolocation = serde_json::json!([
                    "https://1.1.1.1/dns-query",
                    "https://dns.google/dns-query"
                ]);
            }
        }
    }

    if config.get("unified-delay").is_none() {
        config["unified-delay"] = serde_json::json!(true);
    }
    if config.get("tcp-concurrent").is_none() {
        config["tcp-concurrent"] = serde_json::json!(true);
    }

    if let Some(rules) = config.get_mut("rules").and_then(|v| v.as_array_mut()) {
        let has_geolocation_rule = rules.iter().any(|r| {
            r.as_str()
                .map(|s| s.contains("geolocation-!cn"))
                .unwrap_or(false)
        });

        if !has_geolocation_rule {
            if let Some(pos) = rules.iter().position(|r| {
                r.as_str()
                    .map(|s| s.starts_with("GEOIP,"))
                    .unwrap_or(false)
            }) {
                rules.insert(pos, serde_json::json!("GEOSITE,geolocation-!cn,PROXY"));
            }
        }
    }

    tracing::info!("v2 性能优化已应用");
    Ok(())
}

async fn create_default_config(_config_path: &PathBuf) -> Result<()> {
    let default_config = serde_json::json!({
        "config_version": CONFIG_VERSION,
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
        "global-client-fingerprint": "chrome",
        "dns": {
            "enable": true,
            "prefer-h3": true,
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
            "proxy-server-nameserver": [
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
    crate::platform_config::PlatformPaths::config_dir()
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
