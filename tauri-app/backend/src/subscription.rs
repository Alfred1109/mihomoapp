use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use uuid::Uuid;

fn build_subscription_request(
    client: &reqwest::Client,
    subscription: &Subscription,
) -> Result<reqwest::RequestBuilder> {
    let user_agent = subscription
        .user_agent
        .as_deref()
        .unwrap_or("clash");

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Error,
    Updating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStorage {
    pub subscriptions: HashMap<String, Subscription>,
}

pub async fn add_subscription(name: String, url: String, user_agent: Option<String>, use_proxy: bool) -> Result<()> {
    let mut storage = load_subscriptions().await.unwrap_or_default();
    
    let subscription = Subscription {
        id: Uuid::new_v4().to_string(),
        name,
        url,
        user_agent,
        use_proxy,
        created_at: chrono::Utc::now().to_rfc3339(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        proxy_count: 0,
        status: SubscriptionStatus::Active,
        last_error: None,
    };
    
    storage.subscriptions.insert(subscription.id.clone(), subscription);
    save_subscriptions(&storage).await?;
    
    Ok(())
}

pub async fn get_subscriptions() -> Result<Vec<Subscription>> {
    let storage = load_subscriptions().await.unwrap_or_default();
    Ok(storage.subscriptions.values().cloned().collect())
}

pub async fn update_subscription(id: &str) -> Result<()> {
    let mut storage = load_subscriptions().await.unwrap_or_default();
    
    if let Some(subscription) = storage.subscriptions.get_mut(id) {
        subscription.status = SubscriptionStatus::Updating;
        let subscription_clone = subscription.clone();
        save_subscriptions(&storage).await?;
        
        // Fetch and parse subscription content
        match fetch_and_parse_subscription(&subscription_clone).await {
            Ok(proxies) => {
                let proxy_count = proxies.len() as u32;
                
                // Update subscription status
                storage = load_subscriptions().await.unwrap_or_default();
                if let Some(sub) = storage.subscriptions.get_mut(id) {
                    sub.status = SubscriptionStatus::Active;
                    sub.proxy_count = proxy_count;
                    sub.last_updated = chrono::Utc::now().to_rfc3339();
                    sub.last_error = None;
                }
                save_subscriptions(&storage).await?;

                // Regenerate config with all active subscriptions
                let active_ids: Vec<String> = storage.subscriptions.values()
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
                    
                    println!("✓ 订阅更新成功，配置文件已生成");
                    println!("提示: 配置已更新。如果Mihomo服务正在运行，请重启服务以应用更改。");
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
            
            let proxies = fetch_and_parse_subscription(subscription).await?;
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
    
    // Backup current config before generating new one
    if let Err(e) = crate::backup::backup_config().await {
        println!("⚠ 配置备份失败: {}", e);
        // 不阻止配置生成，继续执行
    } else {
        println!("✓ 配置已备份");
    }
    
    // Generate config with proxies
    let mut config = crate::config::load_config().await?;
    
    // 启用nyanpasu的关键优化配置
    config["unified-delay"] = serde_json::json!(true);
    config["tcp-concurrent"] = serde_json::json!(true);
    
    config["proxies"] = serde_json::json!(all_proxies);
    
    // Create or update proxy groups
    // PROXY group: manual selection with auto, all nodes, and DIRECT
    let mut proxy_select_list = vec!["auto".to_string(), "DIRECT".to_string()];
    proxy_select_list.extend(proxy_names.clone());
    
    // auto group: automatic selection based on latency
    let proxy_groups = vec![
        serde_json::json!({
            "name": "PROXY",
            "type": "select",
            "proxies": proxy_select_list
        }),
        serde_json::json!({
            "name": "auto",
            "type": "url-test",
            "proxies": proxy_names.clone(),
            "url": "http://1.1.1.1",
            "interval": 300,
            "tolerance": 50
        })
    ];
    
    config["proxy-groups"] = serde_json::json!(proxy_groups);
    
    // 验证配置
    match crate::validator::validate_config(&config).await {
        Ok(result) => {
            if !result.valid {
                println!("⚠ 配置验证失败:");
                for error in &result.errors {
                    println!("  ✗ {}", error);
                }
                return Err(anyhow::anyhow!("配置验证失败: {}", result.errors.join(", ")));
            }
            
            if !result.warnings.is_empty() {
                println!("⚠ 配置警告:");
                for warning in &result.warnings {
                    println!("  ! {}", warning);
                }
            }
            
            println!("✓ 配置验证通过");
        }
        Err(e) => {
            println!("⚠ 配置验证出错: {}", e);
            // 验证出错不阻止保存，继续执行
        }
    }
    
    crate::config::save_config(config).await?;
    
    Ok(())
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
        return Err(anyhow::anyhow!("HTTP {} when fetching subscription", response.status()));
    }
    
    let content = response.text().await?;
    let proxies = parse_subscription_content(&content)?;
    
    Ok(proxies.len() as u32)
}

async fn fetch_and_parse_subscription(subscription: &Subscription) -> Result<Vec<serde_json::Value>> {
    // 使用真实的浏览器User-Agent避免418错误
    let default_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    let mut client_builder = reqwest::Client::builder()
        .user_agent(subscription.user_agent.as_deref().unwrap_or(default_ua))
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true);
    
    // 如果不使用代理，则完全禁用所有代理
    if !subscription.use_proxy {
        println!("订阅 '{}' 配置为直连模式，完全绕过系统代理", subscription.name);
        // 禁用系统代理和环境变量代理
        client_builder = client_builder.no_proxy();
        
        // 绑定到本地网卡，确保直连
        // 使用 0.0.0.0 绑定到所有本地接口，确保不通过代理
        if let Ok(addr) = "0.0.0.0".parse::<std::net::IpAddr>() {
            client_builder = client_builder.local_address(addr);
        }
    } else {
        println!("订阅 '{}' 配置为使用代理模式", subscription.name);
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
    
    let content = response.text().await
        .context("无法读取订阅内容")?;
    
    if content.is_empty() {
        return Err(anyhow::anyhow!("订阅服务器返回空内容"));
    }
    
    parse_subscription_content(&content)
        .context("订阅内容解析失败")
}

fn parse_subscription_content(content: &str) -> Result<Vec<serde_json::Value>> {
    println!("订阅原始内容长度: {} 字节", content.len());
    println!("订阅内容前100字符: {}", &content[..content.len().min(100)]);
    
    // Try to decode as base64 first
    let decoded_content = if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content.trim()) {
        match String::from_utf8(decoded) {
            Ok(s) => {
                println!("Base64解码成功，解码后长度: {} 字节", s.len());
                s
            }
            Err(_) => {
                println!("Base64解码后无法转换为UTF-8，使用原始内容");
                content.to_string()
            }
        }
    } else {
        println!("不是Base64编码，使用原始内容");
        content.to_string()
    };
    
    // Try parsing as YAML first
    println!("尝试解析为YAML...");
    if let Ok(docs) = yaml_rust::YamlLoader::load_from_str(&decoded_content) {
        println!("YAML解析成功，文档数量: {}", docs.len());
        if let Some(doc) = docs.get(0) {
            // Try to find proxies in the YAML structure
            if let Some(proxies) = doc["proxies"].as_vec() {
                println!("找到proxies字段，代理数量: {}", proxies.len());
                let mut result = Vec::new();
                for (i, proxy) in proxies.iter().enumerate() {
                    match crate::config::yaml_to_json(proxy) {
                        Ok(json_proxy) => {
                            result.push(json_proxy);
                        }
                        Err(e) => {
                            println!("代理 {} 转换失败: {}", i, e);
                        }
                    }
                }
                if !result.is_empty() {
                    println!("成功解析 {} 个代理节点", result.len());
                    return Ok(result);
                } else {
                    println!("proxies字段存在但没有有效节点");
                }
            } else {
                println!("YAML中未找到proxies字段");
            }
        }
    } else {
        println!("YAML解析失败");
    }
    
    // If YAML parsing fails or no proxies found, try parsing individual proxy URLs
    println!("尝试解析为代理URL列表...");
    let lines: Vec<&str> = decoded_content.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    
    println!("找到 {} 行非空内容", lines.len());
    
    let mut proxies = Vec::new();
    let mut errors = Vec::new();
    
    for line in lines {
        match parse_proxy_url(line) {
            Ok(proxy) => proxies.push(proxy),
            Err(e) => {
                // Only log errors for lines that look like proxy URLs
                if line.contains("://") {
                    errors.push(format!("Failed to parse '{}': {}", 
                        &line[..line.len().min(50)], e));
                }
            }
        }
    }
    
    if proxies.is_empty() {
        let error_msg = if errors.is_empty() {
            format!("订阅内容中未找到有效的代理节点。内容预览:\n{}", 
                &decoded_content[..decoded_content.len().min(500)])
        } else {
            format!("订阅内容中未找到有效的代理节点。解析错误:\n{}", errors.join("\n"))
        };
        return Err(anyhow::anyhow!(error_msg));
    }
    
    println!("成功解析 {} 个代理URL", proxies.len());
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
    } else {
        Err(anyhow::anyhow!("Unsupported proxy URL format"))
    }
}

fn parse_shadowsocks_url(url: &str) -> Result<serde_json::Value> {
    // Basic SS URL parsing
    let url = url.strip_prefix("ss://")
        .ok_or_else(|| anyhow::anyhow!("Invalid SS URL: missing ss:// prefix"))?;
    
    // Decode base64 part
    let parts: Vec<&str> = url.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid SS URL format"));
    }
    
    let method_password = String::from_utf8(base64::Engine::decode(&base64::engine::general_purpose::STANDARD, parts[0])?)?;
    let method_pass_parts: Vec<&str> = method_password.splitn(2, ':').collect();
    if method_pass_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid method:password format"));
    }
    
    let server_port_name: Vec<&str> = parts[1].splitn(2, '#').collect();
    let server_port: Vec<&str> = server_port_name[0].splitn(2, ':').collect();
    if server_port.len() != 2 {
        return Err(anyhow::anyhow!("Invalid server:port format"));
    }
    
    let name = if server_port_name.len() > 1 {
        urlencoding::decode(server_port_name[1])?.to_string()
    } else {
        format!("SS-{}", server_port[0])
    };
    
    Ok(serde_json::json!({
        "name": name,
        "type": "ss",
        "server": server_port[0],
        "port": server_port[1].parse::<u16>()?,
        "cipher": method_pass_parts[0],
        "password": method_pass_parts[1]
    }))
}

fn parse_shadowsocksr_url(_url: &str) -> Result<serde_json::Value> {
    // SSR parsing would be more complex, placeholder for now
    Err(anyhow::anyhow!("SSR URL parsing not implemented yet"))
}

fn parse_vmess_url(_url: &str) -> Result<serde_json::Value> {
    // VMess parsing would be more complex, placeholder for now
    Err(anyhow::anyhow!("VMess URL parsing not implemented yet"))
}

fn parse_trojan_url(_url: &str) -> Result<serde_json::Value> {
    // Trojan parsing would be more complex, placeholder for now
    Err(anyhow::anyhow!("Trojan URL parsing not implemented yet"))
}

async fn load_subscriptions() -> Result<SubscriptionStorage> {
    let path = get_subscriptions_path()?;
    
    if !path.exists() {
        return Ok(SubscriptionStorage {
            subscriptions: HashMap::new(),
        });
    }
    
    let content = fs::read_to_string(&path)
        .context("Failed to read subscriptions file")?;
    
    let storage: SubscriptionStorage = serde_json::from_str(&content)
        .context("Failed to parse subscriptions file")?;
    
    Ok(storage)
}

async fn save_subscriptions(storage: &SubscriptionStorage) -> Result<()> {
    let path = get_subscriptions_path()?;
    
    // Ensure data directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create data directory")?;
    }
    
    let content = serde_json::to_string_pretty(storage)
        .context("Failed to serialize subscriptions")?;
    
    fs::write(&path, content)
        .context("Failed to write subscriptions file")?;
    
    Ok(())
}

fn get_subscriptions_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("mihomo")
        .join("data");
    
    Ok(config_dir.join("subscriptions.json"))
}

impl Default for SubscriptionStorage {
    fn default() -> Self {
        Self {
            subscriptions: HashMap::new(),
        }
    }
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
