use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid::Uuid;

fn build_subscription_request(
    client: &reqwest::Client,
    subscription: &Subscription,
) -> Result<reqwest::RequestBuilder> {
    let user_agent = subscription.user_agent.as_deref().unwrap_or("clash");

    let mut builder = client
        .get(&subscription.url)
        .header("User-Agent", user_agent)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive");

    if let Ok(url) = reqwest::Url::parse(&subscription.url) {
        if let Some(host) = url.host_str() {
            builder = builder.header("Host", host);
        }
    }

    Ok(builder)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub user_agent: Option<String>,
    #[serde(default)]
    pub use_proxy: bool,
    pub created_at: String,
    pub last_updated: String,
    pub proxy_count: u32,
    pub status: SubscriptionStatus,
    pub last_error: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Error,
    Updating,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscriptionStorage {
    pub subscriptions: HashMap<String, Subscription>,
}

pub async fn add_subscription(
    name: String,
    url: String,
    user_agent: Option<String>,
    use_proxy: bool,
) -> Result<String> {
    let mut storage = load_subscriptions().await.unwrap_or_default();

    let id = Uuid::new_v4().to_string();
    let file = format!("{}.yaml", id);

    let subscription = Subscription {
        id: id.clone(),
        name,
        url,
        user_agent,
        use_proxy,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        proxy_count: 0,
        status: SubscriptionStatus::Active,
        last_error: None,
        file: Some(file),
    };

    storage
        .subscriptions
        .insert(subscription.id.clone(), subscription);
    save_subscriptions(&storage).await?;

    Ok(id)
}

pub async fn get_subscriptions() -> Result<Vec<Subscription>> {
    let storage = load_subscriptions().await.unwrap_or_default();
    Ok(storage.subscriptions.values().cloned().collect())
}

pub async fn update_subscription(id: &str) -> Result<()> {
    let mut storage = load_subscriptions().await.unwrap_or_default();

    if let Some(subscription) = storage.subscriptions.get_mut(id) {
        subscription.status = SubscriptionStatus::Updating;
        if subscription.file.is_none() {
            subscription.file = Some(format!("{}.yaml", id));
        }
        let subscription_clone = subscription.clone();
        save_subscriptions(&storage).await?;

        match fetch_and_save_subscription(&subscription_clone).await {
            Ok((raw_content, proxies)) => {
                let proxy_count = proxies.len() as u32;

                save_profile_content(id, &raw_content).await?;

                storage = load_subscriptions().await.unwrap_or_default();
                if let Some(sub) = storage.subscriptions.get_mut(id) {
                    sub.status = SubscriptionStatus::Active;
                    sub.proxy_count = proxy_count;
                    sub.last_updated = chrono::Utc::now().to_rfc3339();
                    sub.last_error = None;
                    sub.file = Some(format!("{}.yaml", id));
                }
                save_subscriptions(&storage).await?;

                let active_ids: Vec<String> = storage
                    .subscriptions
                    .values()
                    .filter(|s| s.status == SubscriptionStatus::Active && s.proxy_count > 0)
                    .map(|s| s.id.clone())
                    .collect();

                if !active_ids.is_empty() {
                    if let Err(err) = generate_config_from_subscriptions(active_ids).await {
                        storage = load_subscriptions().await.unwrap_or_default();
                        if let Some(sub) = storage.subscriptions.get_mut(id) {
                            sub.status = SubscriptionStatus::Error;
                            sub.last_error = Some(format!("生成配置失败: {}", err));
                        }
                        save_subscriptions(&storage).await?;
                        return Err(err);
                    }

                    info!("✓ 订阅更新成功，配置文件已生成");
                    info!("提示: 配置已更新。如果Mihomo服务正在运行，请重启服务以应用更改。");
                }
            }
            Err(e) => {
                storage = load_subscriptions().await.unwrap_or_default();
                if let Some(sub) = storage.subscriptions.get_mut(id) {
                    sub.status = SubscriptionStatus::Error;
                    sub.last_error = Some(format!("获取订阅失败: {}", e));
                }
                save_subscriptions(&storage).await?;
                return Err(e);
            }
        }
    } else {
        return Err(anyhow::anyhow!("Subscription not found"));
    }

    Ok(())
}

pub async fn delete_subscription(id: &str) -> Result<()> {
    let mut storage = load_subscriptions().await.unwrap_or_default();

    if storage.subscriptions.remove(id).is_some() {
        save_subscriptions(&storage).await?;

        if let Err(e) = delete_profile_file(id).await {
            warn!("删除订阅文件失败: {}", e);
        }

        Ok(())
    } else {
        Err(anyhow::anyhow!("Subscription not found"))
    }
}

pub async fn generate_config_from_subscriptions(subscription_ids: Vec<String>) -> Result<()> {
    let storage = load_subscriptions().await.unwrap_or_default();
    let mut all_proxies = Vec::new();
    let mut proxy_names = Vec::new();

    for id in subscription_ids {
        if let Some(subscription) = storage.subscriptions.get(&id) {
            if subscription.status != SubscriptionStatus::Active {
                continue;
            }

            let proxies = match load_proxies_from_profile(&id).await {
                Ok(p) => p,
                Err(_) => {
                    info!("本地文件不存在，从网络获取订阅: {}", subscription.name);
                    fetch_and_parse_subscription(subscription).await?
                }
            };

            for proxy in proxies {
                if let Some(name) = proxy["name"].as_str() {
                    proxy_names.push(name.to_string());
                    all_proxies.push(proxy);
                }
            }
        }
    }

    if all_proxies.is_empty() {
        return Err(anyhow::anyhow!("没有找到任何代理节点"));
    }

    let merged_config = crate::base_config::merge_with_proxies(all_proxies, proxy_names).await?;

    crate::config::save_config(merged_config.clone()).await?;

    match crate::validator::validate_config(&merged_config).await {
        Ok(result) => {
            if !result.valid {
                warn!("⚠ 配置验证失败:");
                for error in &result.errors {
                    warn!("  ✗ {}", error);
                }
                return Err(anyhow::anyhow!(
                    "配置验证失败: {}",
                    result.errors.join(", ")
                ));
            }

            if !result.warnings.is_empty() {
                warn!("⚠ 配置警告:");
                for warning in &result.warnings {
                    warn!("  ! {}", warning);
                }
            }

            info!("✓ 配置验证通过");
        }
        Err(e) => {
            warn!("⚠ 配置验证出错: {}", e);
        }
    }

    Ok(())
}

async fn load_proxies_from_profile(id: &str) -> Result<Vec<serde_json::Value>> {
    let content = load_profile_content(id).await?;
    parse_subscription_content(&content)
}

pub async fn export_subscriptions() -> Result<String> {
    let storage = load_subscriptions().await.unwrap_or_default();
    let json_content = serde_json::to_string_pretty(&storage).context("序列化订阅数据失败")?;
    info!("导出 {} 个订阅链接", storage.subscriptions.len());
    Ok(json_content)
}

pub async fn import_subscriptions(json_content: &str) -> Result<u32> {
    let imported_storage: SubscriptionStorage =
        serde_json::from_str(json_content).context("解析导入的订阅数据失败")?;

    if imported_storage.subscriptions.is_empty() {
        return Err(anyhow::anyhow!("导入的数据中没有订阅链接"));
    }

    let subscriptions_path = get_subscriptions_path()?;
    if subscriptions_path.exists() {
        let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
        std::fs::create_dir_all(&backup_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path =
            backup_dir.join(format!("subscriptions_before_import_{}.json", timestamp));
        std::fs::copy(&subscriptions_path, &backup_path)?;
        info!("已备份当前订阅数据");
    }

    let count = imported_storage.subscriptions.len() as u32;
    save_subscriptions(&imported_storage).await?;

    info!("成功导入 {} 个订阅链接", count);
    Ok(count)
}

pub async fn backup_subscriptions() -> Result<String> {
    let subscriptions_path = get_subscriptions_path()?;

    if !subscriptions_path.exists() {
        return Err(anyhow::anyhow!("没有订阅数据可备份"));
    }

    let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
    std::fs::create_dir_all(&backup_dir)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("subscriptions_{}.json", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    std::fs::copy(&subscriptions_path, &backup_path).context("备份订阅数据失败")?;

    info!("订阅数据已备份到: {:?}", backup_path);
    Ok(backup_filename)
}

pub async fn list_subscription_backups() -> Result<Vec<String>> {
    let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in std::fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();

        if filename.starts_with("subscriptions_") && filename.ends_with(".json") {
            backups.push(filename);
        }
    }

    backups.sort_by(|a, b| b.cmp(a));
    Ok(backups)
}

pub async fn restore_subscriptions_from_backup(backup_filename: &str) -> Result<u32> {
    let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
    let backup_path = backup_dir.join(backup_filename);

    if !backup_path.exists() {
        return Err(anyhow::anyhow!("备份文件不存在: {}", backup_filename));
    }

    let content = std::fs::read_to_string(&backup_path).context("读取备份文件失败")?;

    import_subscriptions(&content).await
}

#[allow(dead_code)]
async fn fetch_subscription_content(subscription: &Subscription) -> Result<u32> {
    let client = reqwest::Client::builder()
        .user_agent(subscription.user_agent.as_deref().unwrap_or("clash"))
        .build()?;

    let response = build_subscription_request(&client, subscription)?
        .send()
        .await
        .context("Failed to fetch subscription")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP {} when fetching subscription",
            response.status()
        ));
    }

    let content = response.text().await?;
    let proxies = parse_subscription_content(&content)?;

    Ok(proxies.len() as u32)
}

async fn fetch_and_save_subscription(
    subscription: &Subscription,
) -> Result<(String, Vec<serde_json::Value>)> {
    let default_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    let mut client_builder = reqwest::Client::builder()
        .user_agent(subscription.user_agent.as_deref().unwrap_or(default_ua))
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true);

    if !subscription.use_proxy {
        info!(
            "订阅 '{}' 配置为直连模式，完全绕过系统代理",
            subscription.name
        );
        client_builder = client_builder.no_proxy();

        if let Ok(addr) = "0.0.0.0".parse::<std::net::IpAddr>() {
            client_builder = client_builder.local_address(addr);
        }
    } else {
        info!("订阅 '{}' 配置为使用代理模式", subscription.name);
    }

    let client = client_builder.build()?;

    let response = build_subscription_request(&client, subscription)?
        .send()
        .await
        .context(format!("无法连接到订阅服务器: {}", subscription.url))?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("订阅服务器返回错误: HTTP {}", status));
    }

    let content = response.text().await.context("无法读取订阅内容")?;

    if content.is_empty() {
        return Err(anyhow::anyhow!("订阅服务器返回空内容"));
    }

    let proxies = parse_subscription_content(&content).context("订阅内容解析失败")?;

    let yaml_content = generate_profile_yaml(&content, &proxies)?;

    Ok((yaml_content, proxies))
}

fn generate_profile_yaml(raw_content: &str, proxies: &[serde_json::Value]) -> Result<String> {
    let decoded_content = if let Ok(decoded) = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        raw_content.trim(),
    ) {
        String::from_utf8(decoded).unwrap_or_else(|_| raw_content.to_string())
    } else {
        raw_content.to_string()
    };

    if let Ok(docs) = yaml_rust::YamlLoader::load_from_str(&decoded_content) {
        if let Some(doc) = docs.first() {
            if doc["proxies"].as_vec().is_some() {
                return Ok(decoded_content);
            }
        }
    }

    let proxy_list: Vec<serde_json::Value> = proxies.to_vec();
    let yaml_obj = serde_json::json!({
        "proxies": proxy_list
    });

    serde_yaml::to_string(&yaml_obj).context("Failed to serialize proxies to YAML")
}

async fn fetch_and_parse_subscription(
    subscription: &Subscription,
) -> Result<Vec<serde_json::Value>> {
    let (_, proxies) = fetch_and_save_subscription(subscription).await?;
    Ok(proxies)
}

fn parse_subscription_content(content: &str) -> Result<Vec<serde_json::Value>> {
    debug!("订阅原始内容长度: {} 字节", content.len());
    debug!("订阅内容前100字符: {}", &content[..content.len().min(100)]);

    // Try to decode as base64 first
    let decoded_content = if let Ok(decoded) =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content.trim())
    {
        match String::from_utf8(decoded) {
            Ok(s) => {
                debug!("Base64解码成功，解码后长度: {} 字节", s.len());
                s
            }
            Err(_) => {
                debug!("Base64解码后无法转换为UTF-8，使用原始内容");
                content.to_string()
            }
        }
    } else {
        debug!("不是Base64编码，使用原始内容");
        content.to_string()
    };

    // Try parsing as YAML first
    debug!("尝试解析为YAML...");
    if let Ok(docs) = yaml_rust::YamlLoader::load_from_str(&decoded_content) {
        debug!("YAML解析成功，文档数量: {}", docs.len());
        if let Some(doc) = docs.first() {
            // Try to find proxies in the YAML structure
            if let Some(proxies) = doc["proxies"].as_vec() {
                debug!("找到proxies字段，代理数量: {}", proxies.len());
                let mut result = Vec::new();
                for (i, proxy) in proxies.iter().enumerate() {
                    match crate::config::yaml_to_json(proxy) {
                        Ok(json_proxy) => {
                            result.push(json_proxy);
                        }
                        Err(e) => {
                            debug!("代理 {} 转换失败: {}", i, e);
                        }
                    }
                }
                if !result.is_empty() {
                    info!("成功解析 {} 个代理节点", result.len());
                    return Ok(result);
                } else {
                    debug!("proxies字段存在但没有有效节点");
                }
            } else {
                debug!("YAML中未找到proxies字段");
            }
        }
    } else {
        debug!("YAML解析失败");
    }

    // If YAML parsing fails or no proxies found, try parsing individual proxy URLs
    debug!("尝试解析为代理URL列表...");
    let lines: Vec<&str> = decoded_content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    debug!("找到 {} 行非空内容", lines.len());

    let mut proxies = Vec::new();
    let mut errors = Vec::new();

    for line in lines {
        match parse_proxy_url(line) {
            Ok(proxy) => proxies.push(proxy),
            Err(e) => {
                // Only log errors for lines that look like proxy URLs
                if line.contains("://") {
                    errors.push(format!(
                        "Failed to parse '{}': {}",
                        &line[..line.len().min(50)],
                        e
                    ));
                }
            }
        }
    }

    if proxies.is_empty() {
        let error_msg = if errors.is_empty() {
            format!(
                "订阅内容中未找到有效的代理节点。内容预览:\n{}",
                &decoded_content[..decoded_content.len().min(500)]
            )
        } else {
            format!(
                "订阅内容中未找到有效的代理节点。解析错误:\n{}",
                errors.join("\n")
            )
        };
        return Err(anyhow::anyhow!(error_msg));
    }

    info!("成功解析 {} 个代理URL", proxies.len());
    Ok(proxies)
}

fn parse_proxy_url(url: &str) -> Result<serde_json::Value> {
    let url = url.trim();

    if url.starts_with("ss://") {
        parse_shadowsocks_url(url)
    } else if url.starts_with("ssr://") {
        parse_shadowsocksr_url(url)
    } else if url.starts_with("vmess://") {
        parse_vmess_url(url)
    } else if url.starts_with("trojan://") {
        parse_trojan_url(url)
    } else if url.starts_with("vless://") {
        parse_vless_url(url)
    } else if url.starts_with("hysteria2://") || url.starts_with("hy2://") {
        parse_hysteria2_url(url)
    } else {
        Err(anyhow::anyhow!(
            "Unsupported proxy URL format: {}",
            &url[..url.len().min(20)]
        ))
    }
}

fn parse_shadowsocks_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("ss://")
        .ok_or_else(|| anyhow::anyhow!("Invalid SS URL: missing ss:// prefix"))?;

    let (main_part, name) = if let Some(idx) = url.find('#') {
        let (main, fragment) = url.split_at(idx);
        (
            main.to_string(),
            urlencoding::decode(&fragment[1..])
                .unwrap_or_default()
                .to_string(),
        )
    } else {
        (url.to_string(), String::new())
    };

    let (method, password, server, port) = if main_part.contains('@') {
        let parts: Vec<&str> = main_part.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid SS URL format"));
        }

        let user_info = decode_base64_flexible(parts[0])?;
        let method_pass_parts: Vec<&str> = user_info.splitn(2, ':').collect();
        if method_pass_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid method:password format"));
        }

        let server_port: Vec<&str> = parts[1].splitn(2, ':').collect();
        if server_port.len() != 2 {
            return Err(anyhow::anyhow!("Invalid server:port format"));
        }

        (
            method_pass_parts[0].to_string(),
            method_pass_parts[1].to_string(),
            server_port[0].to_string(),
            server_port[1].parse::<u16>()?,
        )
    } else {
        let decoded = decode_base64_flexible(&main_part)?;
        let parts: Vec<&str> = decoded.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid SS URL format after base64 decode"));
        }

        let method_pass_parts: Vec<&str> = parts[0].splitn(2, ':').collect();
        if method_pass_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid method:password format"));
        }

        let server_port: Vec<&str> = parts[1].splitn(2, ':').collect();
        if server_port.len() != 2 {
            return Err(anyhow::anyhow!("Invalid server:port format"));
        }

        (
            method_pass_parts[0].to_string(),
            method_pass_parts[1].to_string(),
            server_port[0].to_string(),
            server_port[1].parse::<u16>()?,
        )
    };

    let final_name = if name.is_empty() {
        format!("SS-{}", server)
    } else {
        name
    };

    Ok(serde_json::json!({
        "name": final_name,
        "type": "ss",
        "server": server,
        "port": port,
        "cipher": method,
        "password": password
    }))
}

fn parse_shadowsocksr_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("ssr://")
        .ok_or_else(|| anyhow::anyhow!("Invalid SSR URL: missing ssr:// prefix"))?;

    let decoded = decode_base64_flexible(url)?;

    let main_params: Vec<&str> = decoded.splitn(2, "/?").collect();
    let main_part = main_params[0];

    let parts: Vec<&str> = main_part.split(':').collect();
    if parts.len() < 6 {
        return Err(anyhow::anyhow!(
            "Invalid SSR URL format: insufficient parts"
        ));
    }

    let server = parts[0].to_string();
    let port: u16 = parts[1].parse()?;
    let protocol = parts[2].to_string();
    let method = parts[3].to_string();
    let obfs = parts[4].to_string();
    let password_b64 = parts[5..].join(":");
    let password = decode_base64_flexible(&password_b64)?;

    let mut name = format!("SSR-{}", server);
    let mut obfs_param = String::new();
    let mut protocol_param = String::new();

    if main_params.len() > 1 {
        let params_str = main_params[1];
        for param in params_str.split('&') {
            let kv: Vec<&str> = param.splitn(2, '=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "remarks" => {
                        if let Ok(decoded_name) = decode_base64_flexible(kv[1]) {
                            name = decoded_name;
                        }
                    }
                    "obfsparam" => {
                        if let Ok(decoded) = decode_base64_flexible(kv[1]) {
                            obfs_param = decoded;
                        }
                    }
                    "protoparam" => {
                        if let Ok(decoded) = decode_base64_flexible(kv[1]) {
                            protocol_param = decoded;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let mut proxy = serde_json::json!({
        "name": name,
        "type": "ssr",
        "server": server,
        "port": port,
        "cipher": method,
        "password": password,
        "protocol": protocol,
        "obfs": obfs
    });

    if !obfs_param.is_empty() {
        proxy["obfs-param"] = serde_json::json!(obfs_param);
    }
    if !protocol_param.is_empty() {
        proxy["protocol-param"] = serde_json::json!(protocol_param);
    }

    Ok(proxy)
}

fn parse_vmess_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("vmess://")
        .ok_or_else(|| anyhow::anyhow!("Invalid VMess URL: missing vmess:// prefix"))?;

    let decoded = decode_base64_flexible(url)?;
    let config: serde_json::Value =
        serde_json::from_str(&decoded).context("Failed to parse VMess JSON config")?;

    let name = config["ps"]
        .as_str()
        .or_else(|| config["remarks"].as_str())
        .unwrap_or("VMess")
        .to_string();

    let server = config["add"]
        .as_str()
        .or_else(|| config["host"].as_str())
        .ok_or_else(|| anyhow::anyhow!("VMess config missing server address"))?
        .to_string();

    let port: u16 = match &config["port"] {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(443) as u16,
        serde_json::Value::String(s) => s.parse().unwrap_or(443),
        _ => 443,
    };

    let uuid = config["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("VMess config missing UUID"))?
        .to_string();

    let alter_id: u32 = match &config["aid"] {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    };

    let network = config["net"].as_str().unwrap_or("tcp").to_string();

    let tls = match &config["tls"] {
        serde_json::Value::String(s) => s == "tls",
        serde_json::Value::Bool(b) => *b,
        _ => false,
    };

    let mut proxy = serde_json::json!({
        "name": name,
        "type": "vmess",
        "server": server,
        "port": port,
        "uuid": uuid,
        "alterId": alter_id,
        "cipher": config["scy"].as_str().unwrap_or("auto"),
        "tls": tls,
        "skip-cert-verify": true,
        "udp": true
    });

    if let Some(sni) = config["sni"].as_str() {
        if !sni.is_empty() {
            proxy["servername"] = serde_json::json!(sni);
        }
    }

    match network.as_str() {
        "ws" => {
            let mut ws_opts = serde_json::json!({});

            let path = config["path"].as_str().unwrap_or("/");
            ws_opts["path"] = serde_json::json!(path);

            let host = config["host"]
                .as_str()
                .filter(|h| !h.is_empty())
                .unwrap_or(&server);
            ws_opts["headers"] = serde_json::json!({
                "Host": host
            });

            proxy["network"] = serde_json::json!("ws");
            proxy["ws-opts"] = ws_opts;
        }
        "grpc" => {
            proxy["network"] = serde_json::json!("grpc");
            if let Some(path) = config["path"].as_str() {
                proxy["grpc-opts"] = serde_json::json!({
                    "grpc-service-name": path
                });
            }
        }
        "h2" => {
            proxy["network"] = serde_json::json!("h2");
            let mut h2_opts = serde_json::json!({});
            if let Some(path) = config["path"].as_str() {
                h2_opts["path"] = serde_json::json!(path);
            }
            if let Some(host) = config["host"].as_str() {
                h2_opts["host"] = serde_json::json!([host]);
            }
            proxy["h2-opts"] = h2_opts;
        }
        _ => {}
    }

    Ok(proxy)
}

fn parse_trojan_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("trojan://")
        .ok_or_else(|| anyhow::anyhow!("Invalid Trojan URL: missing trojan:// prefix"))?;

    let (main_part, name) = if let Some(idx) = url.find('#') {
        let (main, fragment) = url.split_at(idx);
        (
            main.to_string(),
            urlencoding::decode(&fragment[1..])
                .unwrap_or_default()
                .to_string(),
        )
    } else {
        (url.to_string(), String::new())
    };

    let (password_server, query) = if let Some(idx) = main_part.find('?') {
        let (ps, q) = main_part.split_at(idx);
        (ps.to_string(), q[1..].to_string())
    } else {
        (main_part.clone(), String::new())
    };

    let parts: Vec<&str> = password_server.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid Trojan URL format"));
    }

    let password = urlencoding::decode(parts[0])?.to_string();
    let server_port: Vec<&str> = parts[1].splitn(2, ':').collect();
    if server_port.len() != 2 {
        return Err(anyhow::anyhow!("Invalid server:port format"));
    }

    let server = server_port[0].to_string();
    let port: u16 = server_port[1].parse()?;

    let final_name = if name.is_empty() {
        format!("Trojan-{}", server)
    } else {
        name
    };

    let mut params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !query.is_empty() {
        for param in query.split('&') {
            let kv: Vec<&str> = param.splitn(2, '=').collect();
            if kv.len() == 2 {
                params.insert(
                    kv[0].to_string(),
                    urlencoding::decode(kv[1]).unwrap_or_default().to_string(),
                );
            }
        }
    }

    let mut proxy = serde_json::json!({
        "name": final_name,
        "type": "trojan",
        "server": server,
        "port": port,
        "password": password,
        "skip-cert-verify": true,
        "udp": true
    });

    if let Some(sni) = params.get("sni") {
        if !sni.is_empty() {
            proxy["sni"] = serde_json::json!(sni);
        }
    }

    if let Some(alpn) = params.get("alpn") {
        let alpn_list: Vec<&str> = alpn.split(',').collect();
        proxy["alpn"] = serde_json::json!(alpn_list);
    }

    let network = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");

    match network {
        "ws" => {
            proxy["network"] = serde_json::json!("ws");
            let mut ws_opts = serde_json::json!({});

            if let Some(path) = params.get("path") {
                ws_opts["path"] = serde_json::json!(path);
            }
            if let Some(host) = params.get("host") {
                ws_opts["headers"] = serde_json::json!({
                    "Host": host
                });
            }

            proxy["ws-opts"] = ws_opts;
        }
        "grpc" => {
            proxy["network"] = serde_json::json!("grpc");
            if let Some(service_name) = params.get("serviceName") {
                proxy["grpc-opts"] = serde_json::json!({
                    "grpc-service-name": service_name
                });
            }
        }
        _ => {}
    }

    if params.get("security").map(|s| s.as_str()) == Some("reality") {
        proxy["reality-opts"] = serde_json::json!({
            "public-key": params.get("pbk").unwrap_or(&String::new()),
            "short-id": params.get("sid").unwrap_or(&String::new())
        });
        if let Some(fp) = params.get("fp") {
            proxy["client-fingerprint"] = serde_json::json!(fp);
        }
    }

    Ok(proxy)
}

fn parse_vless_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("vless://")
        .ok_or_else(|| anyhow::anyhow!("Invalid VLESS URL: missing vless:// prefix"))?;

    let (main_part, name) = if let Some(idx) = url.find('#') {
        let (main, fragment) = url.split_at(idx);
        (
            main.to_string(),
            urlencoding::decode(&fragment[1..])
                .unwrap_or_default()
                .to_string(),
        )
    } else {
        (url.to_string(), String::new())
    };

    let (uuid_server, query) = if let Some(idx) = main_part.find('?') {
        let (us, q) = main_part.split_at(idx);
        (us.to_string(), q[1..].to_string())
    } else {
        (main_part.clone(), String::new())
    };

    let parts: Vec<&str> = uuid_server.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid VLESS URL format"));
    }

    let uuid = parts[0].to_string();
    let server_port: Vec<&str> = parts[1].splitn(2, ':').collect();
    if server_port.len() != 2 {
        return Err(anyhow::anyhow!("Invalid server:port format"));
    }

    let server = server_port[0].to_string();
    let port: u16 = server_port[1].parse()?;

    let final_name = if name.is_empty() {
        format!("VLESS-{}", server)
    } else {
        name
    };

    let mut params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !query.is_empty() {
        for param in query.split('&') {
            let kv: Vec<&str> = param.splitn(2, '=').collect();
            if kv.len() == 2 {
                params.insert(
                    kv[0].to_string(),
                    urlencoding::decode(kv[1]).unwrap_or_default().to_string(),
                );
            }
        }
    }

    let mut proxy = serde_json::json!({
        "name": final_name,
        "type": "vless",
        "server": server,
        "port": port,
        "uuid": uuid,
        "skip-cert-verify": true,
        "udp": true
    });

    let flow = params.get("flow").map(|s| s.as_str()).unwrap_or("");
    if !flow.is_empty() {
        proxy["flow"] = serde_json::json!(flow);
    }

    if let Some(sni) = params.get("sni") {
        if !sni.is_empty() {
            proxy["servername"] = serde_json::json!(sni);
        }
    }

    if let Some(alpn) = params.get("alpn") {
        let alpn_list: Vec<&str> = alpn.split(',').collect();
        proxy["alpn"] = serde_json::json!(alpn_list);
    }

    let network = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");

    match network {
        "ws" => {
            proxy["network"] = serde_json::json!("ws");
            let mut ws_opts = serde_json::json!({});

            if let Some(path) = params.get("path") {
                ws_opts["path"] = serde_json::json!(path);
            }
            if let Some(host) = params.get("host") {
                ws_opts["headers"] = serde_json::json!({
                    "Host": host
                });
            }

            proxy["ws-opts"] = ws_opts;
        }
        "grpc" => {
            proxy["network"] = serde_json::json!("grpc");
            if let Some(service_name) = params.get("serviceName") {
                proxy["grpc-opts"] = serde_json::json!({
                    "grpc-service-name": service_name
                });
            }
        }
        "h2" => {
            proxy["network"] = serde_json::json!("h2");
            let mut h2_opts = serde_json::json!({});
            if let Some(path) = params.get("path") {
                h2_opts["path"] = serde_json::json!(path);
            }
            if let Some(host) = params.get("host") {
                h2_opts["host"] = serde_json::json!([host]);
            }
            proxy["h2-opts"] = h2_opts;
        }
        _ => {}
    }

    let security = params.get("security").map(|s| s.as_str()).unwrap_or("");

    match security {
        "tls" => {
            proxy["tls"] = serde_json::json!(true);
        }
        "reality" => {
            proxy["tls"] = serde_json::json!(true);
            proxy["reality-opts"] = serde_json::json!({
                "public-key": params.get("pbk").unwrap_or(&String::new()),
                "short-id": params.get("sid").unwrap_or(&String::new())
            });
            if let Some(fp) = params.get("fp") {
                proxy["client-fingerprint"] = serde_json::json!(fp);
            }
        }
        _ => {}
    }

    Ok(proxy)
}

fn parse_hysteria2_url(url: &str) -> Result<serde_json::Value> {
    let url = url
        .strip_prefix("hysteria2://")
        .or_else(|| url.strip_prefix("hy2://"))
        .ok_or_else(|| anyhow::anyhow!("Invalid Hysteria2 URL"))?;

    let (main_part, name) = if let Some(idx) = url.find('#') {
        let (main, fragment) = url.split_at(idx);
        (
            main.to_string(),
            urlencoding::decode(&fragment[1..])
                .unwrap_or_default()
                .to_string(),
        )
    } else {
        (url.to_string(), String::new())
    };

    let (password_server, query) = if let Some(idx) = main_part.find('?') {
        let (ps, q) = main_part.split_at(idx);
        (ps.to_string(), q[1..].to_string())
    } else {
        (main_part.clone(), String::new())
    };

    let parts: Vec<&str> = password_server.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid Hysteria2 URL format"));
    }

    let password = urlencoding::decode(parts[0])?.to_string();
    let server_port: Vec<&str> = parts[1].splitn(2, ':').collect();
    if server_port.len() != 2 {
        return Err(anyhow::anyhow!("Invalid server:port format"));
    }

    let server = server_port[0].to_string();
    let port: u16 = server_port[1].parse()?;

    let final_name = if name.is_empty() {
        format!("Hysteria2-{}", server)
    } else {
        name
    };

    let mut params: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !query.is_empty() {
        for param in query.split('&') {
            let kv: Vec<&str> = param.splitn(2, '=').collect();
            if kv.len() == 2 {
                params.insert(
                    kv[0].to_string(),
                    urlencoding::decode(kv[1]).unwrap_or_default().to_string(),
                );
            }
        }
    }

    let mut proxy = serde_json::json!({
        "name": final_name,
        "type": "hysteria2",
        "server": server,
        "port": port,
        "password": password,
        "skip-cert-verify": true
    });

    if let Some(sni) = params.get("sni") {
        if !sni.is_empty() {
            proxy["sni"] = serde_json::json!(sni);
        }
    }

    if let Some(obfs) = params.get("obfs") {
        proxy["obfs"] = serde_json::json!(obfs);
        if let Some(obfs_password) = params.get("obfs-password") {
            proxy["obfs-password"] = serde_json::json!(obfs_password);
        }
    }

    if let Some(alpn) = params.get("alpn") {
        let alpn_list: Vec<&str> = alpn.split(',').collect();
        proxy["alpn"] = serde_json::json!(alpn_list);
    }

    Ok(proxy)
}

fn decode_base64_flexible(input: &str) -> Result<String> {
    use base64::Engine;

    let input = input.trim();

    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(input) {
        if let Ok(s) = String::from_utf8(decoded) {
            return Ok(s);
        }
    }

    if let Ok(decoded) = base64::engine::general_purpose::STANDARD_NO_PAD.decode(input) {
        if let Ok(s) = String::from_utf8(decoded) {
            return Ok(s);
        }
    }

    if let Ok(decoded) = base64::engine::general_purpose::URL_SAFE.decode(input) {
        if let Ok(s) = String::from_utf8(decoded) {
            return Ok(s);
        }
    }

    if let Ok(decoded) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input) {
        if let Ok(s) = String::from_utf8(decoded) {
            return Ok(s);
        }
    }

    let sanitized = input.replace('-', "+").replace('_', "/");

    let padded = match sanitized.len() % 4 {
        2 => format!("{}==", sanitized),
        3 => format!("{}=", sanitized),
        _ => sanitized,
    };

    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&padded) {
        if let Ok(s) = String::from_utf8(decoded) {
            return Ok(s);
        }
    }

    Err(anyhow::anyhow!("Failed to decode base64 string"))
}

async fn load_subscriptions() -> Result<SubscriptionStorage> {
    let path = get_subscriptions_path()?;

    if !path.exists() {
        return Ok(SubscriptionStorage {
            subscriptions: HashMap::new(),
        });
    }

    let content = fs::read_to_string(&path).context("Failed to read subscriptions file")?;

    let storage: SubscriptionStorage =
        serde_json::from_str(&content).context("Failed to parse subscriptions file")?;

    Ok(storage)
}

async fn save_subscriptions(storage: &SubscriptionStorage) -> Result<()> {
    let path = get_subscriptions_path()?;

    // Ensure data directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create data directory")?;
    }

    let content =
        serde_json::to_string_pretty(storage).context("Failed to serialize subscriptions")?;

    fs::write(&path, content).context("Failed to write subscriptions file")?;

    Ok(())
}

fn get_subscriptions_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo")
        .join("data");

    Ok(config_dir.join("subscriptions.json"))
}

fn get_profiles_dir() -> Result<PathBuf> {
    let profiles_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo")
        .join("profiles");

    fs::create_dir_all(&profiles_dir).context("Failed to create profiles directory")?;

    Ok(profiles_dir)
}

fn get_profile_path(id: &str) -> Result<PathBuf> {
    let profiles_dir = get_profiles_dir()?;
    Ok(profiles_dir.join(format!("{}.yaml", id)))
}

async fn save_profile_content(id: &str, content: &str) -> Result<()> {
    let path = get_profile_path(id)?;
    fs::write(&path, content).context("Failed to write profile file")?;
    info!("订阅内容已保存到: {:?}", path);
    Ok(())
}

async fn load_profile_content(id: &str) -> Result<String> {
    let path = get_profile_path(id)?;
    if !path.exists() {
        return Err(anyhow::anyhow!("Profile file not found: {:?}", path));
    }
    fs::read_to_string(&path).context("Failed to read profile file")
}

async fn delete_profile_file(id: &str) -> Result<()> {
    let path = get_profile_path(id)?;
    if path.exists() {
        fs::remove_file(&path).context("Failed to delete profile file")?;
        info!("已删除订阅文件: {:?}", path);
    }
    Ok(())
}

impl PartialEq for SubscriptionStatus {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (SubscriptionStatus::Active, SubscriptionStatus::Active)
                | (SubscriptionStatus::Error, SubscriptionStatus::Error)
                | (SubscriptionStatus::Updating, SubscriptionStatus::Updating)
        )
    }
}
