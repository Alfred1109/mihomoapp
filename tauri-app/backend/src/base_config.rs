use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

const BASE_CONFIG_VERSION: u32 = 2;

pub fn get_base_config_path() -> Result<PathBuf> {
    let config_dir = crate::platform_config::PlatformPaths::config_dir()?;
    Ok(config_dir.join("base_config.yaml"))
}

pub async fn load_base_config() -> Result<Value> {
    let base_config_path = get_base_config_path()?;

    if !base_config_path.exists() {
        info!("基础配置不存在，创建默认基础配置");
        let default_config = create_default_base_config();
        save_base_config(&default_config).await?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&base_config_path)
        .context("无法读取基础配置文件")?;

    let yaml_docs = yaml_rust::YamlLoader::load_from_str(&content)
        .context("解析基础配置 YAML 失败")?;

    if yaml_docs.is_empty() {
        warn!("基础配置文件为空，使用默认配置");
        let default_config = create_default_base_config();
        save_base_config(&default_config).await?;
        return Ok(default_config);
    }

    let config = crate::config::yaml_to_json(&yaml_docs[0])
        .context("转换基础配置为 JSON 失败")?;

    info!("基础配置加载成功");
    Ok(config)
}

pub async fn save_base_config(config: &Value) -> Result<()> {
    let base_config_path = get_base_config_path()?;

    if let Some(parent) = base_config_path.parent() {
        fs::create_dir_all(parent).context("创建配置目录失败")?;
    }

    let yaml_value: serde_yaml::Value = serde_json::from_value(config.clone())
        .context("转换配置格式失败")?;

    let yaml_content = serde_yaml::to_string(&yaml_value)
        .context("序列化 YAML 失败")?;

    fs::write(&base_config_path, yaml_content)
        .context("写入基础配置失败")?;

    info!("基础配置已保存: {:?}", base_config_path);
    Ok(())
}

pub async fn reset_to_default() -> Result<()> {
    info!("重置基础配置为默认值");
    
    let base_config_path = get_base_config_path()?;
    if base_config_path.exists() {
        let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
        fs::create_dir_all(&backup_dir)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("base_config_before_reset_{}.yaml", timestamp));
        fs::copy(&base_config_path, &backup_path)
            .context("备份当前基础配置失败")?;
        info!("已备份当前基础配置到: {:?}", backup_path);
    }

    let default_config = create_default_base_config();
    save_base_config(&default_config).await?;

    info!("基础配置已重置为默认值");
    Ok(())
}

pub fn create_default_base_config() -> Value {
    serde_json::json!({
        "base_config_version": BASE_CONFIG_VERSION,
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
    })
}

pub async fn merge_with_proxies(proxies: Vec<Value>, proxy_names: Vec<String>) -> Result<Value> {
    let mut config = load_base_config().await?;

    config["proxies"] = serde_json::json!(proxies);

    let mut proxy_select_list = vec!["auto".to_string(), "DIRECT".to_string()];
    proxy_select_list.extend(proxy_names.clone());

    let proxy_groups = vec![
        serde_json::json!({
            "name": "PROXY",
            "type": "select",
            "proxies": proxy_select_list
        }),
        serde_json::json!({
            "name": "auto",
            "type": "url-test",
            "proxies": proxy_names,
            "url": "http://www.gstatic.com/generate_204",
            "interval": 300,
            "tolerance": 50
        }),
    ];

    config["proxy-groups"] = serde_json::json!(proxy_groups);

    if config.get("base_config_version").is_some() {
        if let Some(obj) = config.as_object_mut() {
            obj.remove("base_config_version");
        }
    }

    config["config_version"] = serde_json::json!(2);

    info!("已合并基础配置和代理节点");
    Ok(config)
}

pub async fn export_base_config() -> Result<String> {
    let config = load_base_config().await?;
    let yaml_value: serde_yaml::Value = serde_json::from_value(config)
        .context("转换配置格式失败")?;
    let yaml_content = serde_yaml::to_string(&yaml_value)
        .context("序列化 YAML 失败")?;
    Ok(yaml_content)
}

pub async fn import_base_config(yaml_content: &str) -> Result<()> {
    let yaml_docs = yaml_rust::YamlLoader::load_from_str(yaml_content)
        .context("解析导入的 YAML 失败")?;

    if yaml_docs.is_empty() {
        return Err(anyhow::anyhow!("导入的配置为空"));
    }

    let config = crate::config::yaml_to_json(&yaml_docs[0])
        .context("转换配置格式失败")?;

    if config.get("proxies").is_some() || config.get("proxy-groups").is_some() {
        warn!("导入的配置包含代理节点，将被忽略");
    }

    let mut clean_config = config.clone();
    if let Some(obj) = clean_config.as_object_mut() {
        obj.remove("proxies");
        obj.remove("proxy-groups");
    }

    let base_config_path = get_base_config_path()?;
    if base_config_path.exists() {
        let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
        fs::create_dir_all(&backup_dir)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("base_config_before_import_{}.yaml", timestamp));
        fs::copy(&base_config_path, &backup_path)?;
        info!("已备份当前基础配置");
    }

    save_base_config(&clean_config).await?;
    info!("基础配置导入成功");
    Ok(())
}
