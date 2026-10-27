use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tracing::{debug, info};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    update_token: Option<String>,
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
    let id = Uuid::new_v4().to_string();
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
        file: Some(format!("{}.yaml", id)),
        update_token: None,
    };
    mutate_store(move |_, storage| {
        storage
            .subscriptions
            .insert(subscription.id.clone(), subscription);
        Ok(())
    })
    .await?;
    Ok(id)
}

pub async fn get_subscriptions() -> Result<Vec<Subscription>> {
    Ok(load_subscriptions()
        .await?
        .subscriptions
        .values()
        .cloned()
        .collect())
}

pub async fn update_subscription(id: &str) -> Result<()> {
    let id = id.to_string();
    let token = Uuid::new_v4().to_string();
    let request_id = id.clone();
    let request_token = token.clone();
    let subscription = mutate_store(move |_, storage| {
        let subscription = storage
            .subscriptions
            .get_mut(&request_id)
            .context("Subscription not found")?;
        subscription.status = SubscriptionStatus::Updating;
        subscription.update_token = Some(request_token);
        Ok(subscription.clone())
    })
    .await?;

    // No storage lock is held while downloading or parsing the subscription.
    let fetched = fetch_and_save_subscription(&subscription).await;
    let (raw_content, proxies) = match fetched {
        Ok(result) => result,
        Err(error) => {
            set_update_error(id, token, format!("获取订阅失败: {}", error)).await?;
            return Err(error);
        }
    };
    let commit_id = id.clone();
    let commit_token = token.clone();
    let committed =
        commit_profile(commit_id, commit_token, raw_content, proxies.len() as u32).await;
    if let Err(error) = committed {
        // A newer operation owns the subscription if its token changed.
        let _ = set_update_error(id, token, format!("保存订阅失败: {}", error)).await;
        return Err(error);
    }

    if let Err(error) = generate_subscription_config(None).await {
        set_update_error(id, token, format!("生成配置失败: {}", error)).await?;
        return Err(error);
    }
    mutate_store(move |_, storage| {
        let sub = matching_update(storage, &id, &token)?;
        sub.update_token = None;
        Ok(())
    })
    .await?;
    info!("✓ 订阅更新成功，配置文件已生成");
    Ok(())
}

fn matching_update<'a>(
    storage: &'a mut SubscriptionStorage,
    id: &str,
    token: &str,
) -> Result<&'a mut Subscription> {
    let sub = storage
        .subscriptions
        .get_mut(id)
        .context("Subscription was deleted during update")?;
    if sub.update_token.as_deref() != Some(token) {
        return Err(anyhow::anyhow!("订阅更新已被较新的操作替代"));
    }
    Ok(sub)
}

async fn set_update_error(id: String, token: String, message: String) -> Result<()> {
    mutate_store(move |_, storage| {
        let sub = matching_update(storage, &id, &token)?;
        sub.status = SubscriptionStatus::Error;
        sub.last_error = Some(message);
        sub.update_token = None;
        Ok(())
    })
    .await
}

async fn commit_profile(
    id: String,
    token: String,
    content: String,
    proxy_count: u32,
) -> Result<()> {
    with_store(true, move |store| {
        store.commit_profile(&id, &token, &content, proxy_count)
    })
    .await
}

pub async fn delete_subscription(id: &str) -> Result<()> {
    let id = id.to_string();
    with_store(true, move |store| {
        let mut storage = store.read()?;
        storage
            .subscriptions
            .remove(&id)
            .context("Subscription not found")?;
        // Keep historical profiles: existing JSON backups may still reference them.
        store.write(&storage)
    })
    .await
}

static CONFIG_GENERATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn generate_config_from_subscriptions(subscription_ids: Vec<String>) -> Result<()> {
    generate_subscription_config(Some(subscription_ids)).await
}

async fn generate_subscription_config(subscription_ids: Option<Vec<String>>) -> Result<()> {
    // Serialize builders in this process; the metadata comparison below also protects
    // against updates performed by another application process.
    let _generation_guard = CONFIG_GENERATION_LOCK.lock().await;
    for _ in 0..5 {
        let selected_ids = subscription_ids.clone();
        let (revision, profiles) = with_store(false, move |store| {
            let storage = store.read()?;
            let revision = serde_json::to_value(&storage)?;
            let ids = selected_ids.unwrap_or_else(|| {
                storage
                    .subscriptions
                    .values()
                    .filter(|sub| sub.status == SubscriptionStatus::Active && sub.proxy_count > 0)
                    .map(|sub| sub.id.clone())
                    .collect()
            });
            let mut profiles = Vec::new();
            for id in ids {
                if let Some(subscription) = storage.subscriptions.get(&id) {
                    if subscription.status == SubscriptionStatus::Active {
                        let content = store
                            .profile_path(subscription)
                            .and_then(|path| fs::read_to_string(path).context("读取订阅缓存失败"));
                        profiles.push((subscription.clone(), content));
                    }
                }
            }
            Ok((revision, profiles))
        })
        .await?;
        let mut all_proxies = Vec::new();
        let mut proxy_names = Vec::new();
        for (subscription, cached_content) in profiles {
            let proxies =
                match cached_content.and_then(|content| parse_subscription_content(&content)) {
                    Ok(proxies) => proxies,
                    Err(_) => {
                        info!("本地文件不可用，从网络获取订阅: {}", subscription.name);
                        fetch_and_parse_subscription(&subscription).await?
                    }
                };
            for proxy in proxies {
                if let Some(name) = proxy["name"].as_str() {
                    proxy_names.push(name.to_string());
                    all_proxies.push(proxy);
                }
            }
        }
        if all_proxies.is_empty() {
            return Err(anyhow::anyhow!("没有找到任何代理节点"));
        }
        let merged_config =
            crate::base_config::merge_with_proxies(all_proxies, proxy_names).await?;
        let committed = with_store(true, move |store| {
            store.commit_if_current(&revision, || {
                // The file lock spans verification and the final config commit,
                // never a download. ConfigManager does not acquire this lock.
                tokio::runtime::Handle::current()
                    .block_on(crate::config::save_validated_config(merged_config))
            })
        })
        .await?;
        if committed {
            return Ok(());
        }
    }
    Err(anyhow::anyhow!("订阅持续变更，未覆盖当前配置，请稍后重试"))
}

pub async fn export_subscriptions() -> Result<String> {
    let storage = load_subscriptions().await?;
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

    let count = imported_storage.subscriptions.len() as u32;
    let mut imported_storage = imported_storage;
    for (id, subscription) in &mut imported_storage.subscriptions {
        if id != &subscription.id {
            return Err(anyhow::anyhow!("订阅 ID 与存储键不一致"));
        }
        subscription.update_token = None;
    }
    with_store(true, move |store| {
        // Explicit import may replace a damaged store, but always backs up its raw bytes first.
        for subscription in imported_storage.subscriptions.values() {
            store.profile_path(subscription)?;
        }
        if store.path.exists() {
            let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
            store.backup_to(&backup_dir, "subscriptions_before_import")?;
        }
        store.write(&imported_storage)
    })
    .await?;
    info!("成功导入 {} 个订阅链接", count);
    Ok(count)
}

pub async fn backup_subscriptions() -> Result<String> {
    with_store(false, |store| {
        if !store.path.exists() {
            return Err(anyhow::anyhow!("没有订阅数据可备份"));
        }
        let backup_dir = crate::platform_config::PlatformPaths::backup_dir()?;
        store.backup_to(&backup_dir, "subscriptions")
    })
    .await
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
        .timeout(std::time::Duration::from_secs(30));

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
        .map_err(reqwest::Error::without_url)
        .context("无法连接到订阅服务器")?;

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

    for (index, line) in lines.into_iter().enumerate() {
        match parse_proxy_url(line) {
            Ok(proxy) => proxies.push(proxy),
            Err(e) => {
                // Only log errors for lines that look like proxy URLs
                if line.contains("://") {
                    errors.push(format!("第 {} 行代理解析失败: {}", index + 1, e));
                }
            }
        }
    }

    if proxies.is_empty() {
        let error_msg = if errors.is_empty() {
            "订阅内容中未找到有效的代理节点".to_string()
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
        Err(anyhow::anyhow!("Unsupported proxy URL format"))
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

// Only explicit subscription settings may disable certificate verification.
// Reject contradictory aliases instead of silently weakening a secure setting.
fn parse_skip_cert_verify<'a>(
    settings: impl IntoIterator<Item = (&'a str, &'a serde_json::Value)>,
) -> Result<bool> {
    let mut selected = None;
    for (key, value) in settings {
        let enabled = match value {
            serde_json::Value::Bool(value) => *value,
            serde_json::Value::Number(value) if value.as_u64() == Some(1) => true,
            serde_json::Value::Number(value) if value.as_u64() == Some(0) => false,
            serde_json::Value::String(value) => match value.as_str() {
                "1" | "true" => true,
                "0" | "false" => false,
                _ => return Err(anyhow::anyhow!("Invalid TLS boolean for {}", key)),
            },
            _ => return Err(anyhow::anyhow!("Invalid TLS boolean for {}", key)),
        };
        if selected.is_some_and(|previous| previous != enabled) {
            return Err(anyhow::anyhow!(
                "Conflicting TLS certificate verification settings"
            ));
        }
        selected = Some(enabled);
    }
    Ok(selected.unwrap_or(false))
}

fn query_skip_cert_verify(params: &HashMap<String, String>) -> Result<bool> {
    let settings: Vec<(&str, serde_json::Value)> =
        ["allowInsecure", "insecure", "skip-cert-verify"]
            .iter()
            .filter_map(|key| {
                params
                    .get(*key)
                    .map(|value| (*key, serde_json::Value::String(value.clone())))
            })
            .collect();
    parse_skip_cert_verify(settings.iter().map(|(key, value)| (*key, value)))
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
        "skip-cert-verify": parse_skip_cert_verify(
            ["allowInsecure", "insecure", "skip-cert-verify"]
                .iter()
                .filter_map(|key| config.get(*key).map(|value| (*key, value)))
        )?,
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
        "skip-cert-verify": query_skip_cert_verify(&params)?,
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
        "skip-cert-verify": query_skip_cert_verify(&params)?,
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
        "skip-cert-verify": query_skip_cert_verify(&params)?
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

struct SubscriptionStore {
    path: PathBuf,
    profiles_dir: PathBuf,
}

impl SubscriptionStore {
    fn lock<T>(&self, exclusive: bool, operation: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        let parent = self
            .path
            .parent()
            .context("Missing subscription data directory")?;
        fs::create_dir_all(parent).context("Failed to create subscription data directory")?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("lock"))
            .context("Failed to open subscription lock")?;
        if exclusive {
            FileExt::lock_exclusive(&lock)?;
        } else {
            FileExt::lock_shared(&lock)?;
        }
        let result = operation(self);
        let _ = FileExt::unlock(&lock);
        result
    }

    fn read(&self) -> Result<SubscriptionStorage> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SubscriptionStorage::default())
            }
            Err(error) => return Err(error).context("Failed to read subscriptions file"),
        };
        serde_json::from_str(&content).context("订阅数据损坏，已保留原文件；请从备份恢复")
    }

    fn write(&self, storage: &SubscriptionStorage) -> Result<()> {
        atomic_write(&self.path, serde_json::to_vec_pretty(storage)?.as_slice())
    }

    fn transaction<T>(
        &self,
        operation: impl FnOnce(&Self, &mut SubscriptionStorage) -> Result<T>,
    ) -> Result<T> {
        let mut storage = self.read()?;
        let result = operation(self, &mut storage)?;
        self.write(&storage)?;
        Ok(result)
    }

    fn commit_if_current(
        &self,
        revision: &serde_json::Value,
        commit: impl FnOnce() -> Result<()>,
    ) -> Result<bool> {
        if serde_json::to_value(self.read()?)? != *revision {
            return Ok(false);
        }
        commit()?;
        Ok(true)
    }

    fn profile_path(&self, subscription: &Subscription) -> Result<PathBuf> {
        let filename = subscription
            .file
            .clone()
            .unwrap_or_else(|| format!("{}.yaml", subscription.id));
        let mut components = Path::new(&filename).components();
        if !matches!(components.next(), Some(Component::Normal(_)))
            || components.next().is_some()
            || filename.contains('\\')
        {
            return Err(anyhow::anyhow!("Invalid subscription profile filename"));
        }
        Ok(self.profiles_dir.join(filename))
    }

    fn commit_profile(&self, id: &str, token: &str, content: &str, proxy_count: u32) -> Result<()> {
        self.commit_profile_using(id, token, content, proxy_count, |storage| {
            self.write(storage)
        })
    }

    fn commit_profile_using(
        &self,
        id: &str,
        token: &str,
        content: &str,
        proxy_count: u32,
        persist: impl FnOnce(&SubscriptionStorage) -> Result<()>,
    ) -> Result<()> {
        let mut storage = self.read()?;
        let sub = matching_update(&mut storage, id, token)?;
        // Immutable candidates make the metadata rename the single commit point.
        sub.file = Some(format!("{}.yaml", Uuid::new_v4()));
        let candidate = self.profile_path(sub)?;
        atomic_write(&candidate, content.as_bytes())?;
        sub.status = SubscriptionStatus::Active;
        sub.proxy_count = proxy_count;
        sub.last_updated = chrono::Utc::now().to_rfc3339();
        sub.last_error = None;
        if let Err(error) = persist(&storage) {
            let _ = fs::remove_file(candidate);
            return Err(error);
        }
        // Historical profiles are retained because metadata backups reference them.
        // Garbage collection must first enumerate live and backup references.
        Ok(())
    }

    fn backup_to(&self, backup_dir: &Path, prefix: &str) -> Result<String> {
        fs::create_dir_all(backup_dir)?;
        let filename = format!(
            "{}_{}_{}.json",
            prefix,
            chrono::Utc::now().format("%Y%m%d_%H%M%S"),
            Uuid::new_v4()
        );
        let content = fs::read(&self.path).context("读取待备份订阅失败")?;
        atomic_write(&backup_dir.join(&filename), &content)?;
        Ok(filename)
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().context("Missing file directory")?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".subscription-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path).context("Failed to replace subscription file")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

async fn with_store<T: Send + 'static>(
    exclusive: bool,
    operation: impl FnOnce(&SubscriptionStore) -> Result<T> + Send + 'static,
) -> Result<T> {
    // Blocking filesystem locks never occupy a Tokio runtime worker or span a network await.
    tokio::task::spawn_blocking(move || {
        let store = SubscriptionStore {
            path: get_subscriptions_path()?,
            profiles_dir: get_profiles_dir()?,
        };
        store.lock(exclusive, operation)
    })
    .await
    .context("Subscription storage task failed")?
}

async fn mutate_store<T: Send + 'static>(
    operation: impl FnOnce(&SubscriptionStore, &mut SubscriptionStorage) -> Result<T> + Send + 'static,
) -> Result<T> {
    with_store(true, move |store| store.transaction(operation)).await
}

async fn load_subscriptions() -> Result<SubscriptionStorage> {
    with_store(false, SubscriptionStore::read).await
}

fn get_subscriptions_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo")
        .join("data")
        .join("subscriptions.json"))
}

fn get_profiles_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo")
        .join("profiles"))
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

#[cfg(test)]
mod tls_tests {
    use super::*;
    use base64::Engine;

    fn vmess_url(setting: Option<(&str, serde_json::Value)>) -> String {
        let mut config = serde_json::json!({
            "add": "example.com", "port": 443,
            "id": "00000000-0000-0000-0000-000000000001", "tls": "tls"
        });
        if let Some((key, value)) = setting {
            config[key] = value;
        }
        format!(
            "vmess://{}",
            base64::engine::general_purpose::STANDARD.encode(config.to_string())
        )
    }

    #[test]
    fn imported_links_verify_certificates_by_default() {
        let urls = [
            vmess_url(None),
            "trojan://password@example.com:443".to_string(),
            "vless://id@example.com:443?security=tls".to_string(),
            "hy2://password@example.com:443".to_string(),
        ];
        for url in urls {
            assert_eq!(parse_proxy_url(&url).unwrap()["skip-cert-verify"], false);
        }
    }

    #[test]
    fn imported_links_preserve_explicit_verification_settings() {
        for (value, expected) in [("1", true), ("0", false), ("true", true), ("false", false)] {
            for prefix in ["trojan://password", "vless://id", "hy2://password"] {
                let url = format!("{}@example.com:443?insecure={}", prefix, value);
                assert_eq!(parse_proxy_url(&url).unwrap()["skip-cert-verify"], expected);
            }
        }
        for value in [
            serde_json::json!(true),
            serde_json::json!(1),
            serde_json::json!("true"),
        ] {
            assert_eq!(
                parse_proxy_url(&vmess_url(Some(("allowInsecure", value)))).unwrap()
                    ["skip-cert-verify"],
                true
            );
        }
        assert_eq!(
            parse_proxy_url("trojan://password@example.com:443?allowInsecure=1").unwrap()
                ["skip-cert-verify"],
            true
        );
        assert_eq!(
            parse_proxy_url("vless://id@example.com:443?skip-cert-verify=1").unwrap()
                ["skip-cert-verify"],
            true
        );
    }

    #[test]
    fn invalid_or_conflicting_verification_settings_are_rejected() {
        for query in ["insecure=yes", "insecure=2", "insecure=1&allowInsecure=0"] {
            assert!(parse_proxy_url(&format!("hy2://password@example.com:443?{}", query)).is_err());
        }
        assert!(parse_proxy_url(&vmess_url(Some((
            "allowInsecure",
            serde_json::json!("yes")
        ))))
        .is_err());
    }

    #[test]
    fn yaml_subscriptions_keep_explicit_verification_settings() {
        let proxies = parse_subscription_content("proxies:\n  - name: trusted-custom\n    type: trojan\n    server: example.com\n    port: 443\n    password: password\n    skip-cert-verify: true\n").unwrap();
        assert_eq!(proxies[0]["skip-cert-verify"], true);
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    struct TestStore {
        store: SubscriptionStore,
        root: PathBuf,
    }
    impl TestStore {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("mihomo-subscriptions-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            Self {
                store: SubscriptionStore {
                    path: root.join("data/subscriptions.json"),
                    profiles_dir: root.join("profiles"),
                },
                root,
            }
        }
    }
    impl Drop for TestStore {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn subscription(id: &str, token: &str) -> Subscription {
        Subscription {
            id: id.into(),
            name: id.into(),
            url: "https://example.com/subscription".into(),
            user_agent: None,
            use_proxy: false,
            created_at: "created".into(),
            last_updated: "updated".into(),
            proxy_count: 1,
            status: SubscriptionStatus::Updating,
            last_error: None,
            file: Some(format!("{}.yaml", id)),
            update_token: Some(token.into()),
        }
    }

    #[test]
    fn missing_store_is_empty_but_damaged_store_is_never_overwritten() {
        let test = TestStore::new();
        assert!(test
            .store
            .lock(false, SubscriptionStore::read)
            .unwrap()
            .subscriptions
            .is_empty());
        fs::write(&test.store.path, "damaged data").unwrap();
        let result = test.store.lock(true, |store| {
            store.transaction(|_, storage| {
                storage
                    .subscriptions
                    .insert("new".into(), subscription("new", "token"));
                Ok(())
            })
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(&test.store.path).unwrap(),
            "damaged data"
        );
    }

    #[test]
    fn concurrent_transactions_keep_every_independent_subscription() {
        let test = TestStore::new();
        let barrier = Arc::new(Barrier::new(12));
        let mut threads = Vec::new();
        for index in 0..12 {
            let store = SubscriptionStore {
                path: test.store.path.clone(),
                profiles_dir: test.store.profiles_dir.clone(),
            };
            let barrier = barrier.clone();
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                store
                    .lock(true, |store| {
                        store.transaction(|_, storage| {
                            let id = index.to_string();
                            storage
                                .subscriptions
                                .insert(id.clone(), subscription(&id, "token"));
                            Ok(())
                        })
                    })
                    .unwrap();
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(
            test.store
                .lock(false, SubscriptionStore::read)
                .unwrap()
                .subscriptions
                .len(),
            12
        );
    }

    #[test]
    fn stale_and_deleted_updates_cannot_replace_profiles_or_resurrect_entries() {
        let test = TestStore::new();
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage
                        .subscriptions
                        .insert("node".into(), subscription("node", "newer"));
                    Ok(())
                })
            })
            .unwrap();
        assert!(test
            .store
            .lock(true, |store| store
                .commit_profile("node", "older", "stale", 2))
            .is_err());
        assert!(!test.store.profiles_dir.exists());
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage.subscriptions.remove("node");
                    Ok(())
                })
            })
            .unwrap();
        assert!(test
            .store
            .lock(true, |store| store
                .commit_profile("node", "newer", "deleted", 2))
            .is_err());
        assert!(test.store.read().unwrap().subscriptions.is_empty());
        assert!(!test.store.profiles_dir.exists());
    }

    #[test]
    fn metadata_commit_failure_keeps_old_profile_and_discards_candidate() {
        let test = TestStore::new();
        let old = subscription("node", "token");
        atomic_write(&test.store.profile_path(&old).unwrap(), b"old profile").unwrap();
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage.subscriptions.insert("node".into(), old.clone());
                    Ok(())
                })
            })
            .unwrap();
        let original = fs::read(&test.store.path).unwrap();
        let result = test.store.lock(true, |store| {
            store.commit_profile_using("node", "token", "new profile", 2, |_| {
                Err(anyhow::anyhow!("injected metadata failure"))
            })
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&test.store.path).unwrap(), original);
        assert_eq!(
            fs::read_to_string(test.store.profile_path(&old).unwrap()).unwrap(),
            "old profile"
        );
        assert_eq!(fs::read_dir(&test.store.profiles_dir).unwrap().count(), 1);
    }

    #[test]
    fn profile_commit_switches_reference_and_content_together() {
        let test = TestStore::new();
        let old = subscription("node", "token");
        atomic_write(&test.store.profile_path(&old).unwrap(), b"old profile").unwrap();
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage.subscriptions.insert("node".into(), old.clone());
                    Ok(())
                })
            })
            .unwrap();
        test.store
            .lock(true, |store| {
                store.commit_profile("node", "token", "new profile", 2)
            })
            .unwrap();
        test.store
            .lock(false, |store| {
                let storage = store.read()?;
                let updated = &storage.subscriptions["node"];
                assert_eq!(updated.proxy_count, 2);
                assert!(updated.status == SubscriptionStatus::Active);
                assert_eq!(
                    fs::read_to_string(store.profile_path(updated)?).unwrap(),
                    "new profile"
                );
                assert!(store.profile_path(&old)?.exists());
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn stale_configuration_snapshot_never_reaches_commit() {
        let test = TestStore::new();
        let revision = serde_json::to_value(test.store.read().unwrap()).unwrap();
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage
                        .subscriptions
                        .insert("new".into(), subscription("new", "token"));
                    Ok(())
                })
            })
            .unwrap();
        let committed = test
            .store
            .lock(true, |store| {
                store.commit_if_current(&revision, || panic!("stale configuration committed"))
            })
            .unwrap();
        assert!(!committed);
        let latest = serde_json::to_value(test.store.read().unwrap()).unwrap();
        let mut reached_commit = false;
        assert!(test
            .store
            .lock(true, |store| store.commit_if_current(&latest, || {
                reached_commit = true;
                Ok(())
            }))
            .unwrap());
        assert!(reached_commit);
    }

    #[test]
    fn existing_metadata_backup_keeps_its_profile_after_later_updates() {
        let test = TestStore::new();
        let old = subscription("node", "token");
        atomic_write(
            &test.store.profile_path(&old).unwrap(),
            b"backed-up profile",
        )
        .unwrap();
        test.store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    storage.subscriptions.insert("node".into(), old.clone());
                    Ok(())
                })
            })
            .unwrap();
        let backup_dir = test.root.join("backups");
        let backup = test
            .store
            .lock(false, |store| store.backup_to(&backup_dir, "subscriptions"))
            .unwrap();
        test.store
            .lock(true, |store| {
                store.commit_profile("node", "token", "current profile", 2)
            })
            .unwrap();
        let restored: SubscriptionStorage =
            serde_json::from_slice(&fs::read(backup_dir.join(backup)).unwrap()).unwrap();
        assert_eq!(
            fs::read_to_string(
                test.store
                    .profile_path(&restored.subscriptions["node"])
                    .unwrap()
            )
            .unwrap(),
            "backed-up profile"
        );
    }

    #[test]
    fn non_ascii_subscription_content_does_not_slice_inside_utf8() {
        let content = "中文订阅内容".repeat(30);
        assert!(parse_subscription_content(&content).is_err());
    }
    #[test]
    fn storage_lock_worker() {
        let Some(root) = std::env::var_os("MIHOMO_SUBSCRIPTION_LOCK_TEST_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let store = SubscriptionStore {
            path: root.join("data/subscriptions.json"),
            profiles_dir: root.join("profiles"),
        };
        fs::write(root.join("writer-ready"), b"ready").unwrap();
        store
            .lock(true, |store| {
                store.transaction(|_, storage| {
                    assert!(
                        root.join("config-committed").exists(),
                        "writer entered before configuration commit completed"
                    );
                    storage
                        .subscriptions
                        .insert("child".into(), subscription("child", "token"));
                    Ok(())
                })
            })
            .unwrap();
    }

    #[test]
    fn configuration_check_and_commit_exclude_cross_process_mutations() {
        let test = TestStore::new();
        let revision = serde_json::to_value(test.store.read().unwrap()).unwrap();
        let mut child = None;
        test.store
            .lock(true, |store| {
                store.commit_if_current(&revision, || {
                    let process = std::process::Command::new(std::env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "subscription::storage_tests::storage_lock_worker",
                        ])
                        .env("MIHOMO_SUBSCRIPTION_LOCK_TEST_ROOT", &test.root)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                        .unwrap();
                    child = Some(process);
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                    while !test.root.join("writer-ready").exists() {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "child writer did not become ready"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    assert!(child.as_mut().unwrap().try_wait().unwrap().is_none());
                    fs::write(test.root.join("config-committed"), b"committed").unwrap();
                    Ok(())
                })
            })
            .unwrap();
        assert!(child.unwrap().wait().unwrap().success());
        assert!(test
            .store
            .read()
            .unwrap()
            .subscriptions
            .contains_key("child"));
    }
}
