use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn build_subscription_request(
    client: &reqwest::Client,
    subscription: &Subscription,
) -> Result<reqwest::RequestBuilder> {
    let user_agent = subscription
        .user_agent
        .as_deref()
        .unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    let mut builder = client
        .get(&subscription.url)
        .header("User-Agent", user_agent)
        .header("Accept", "*/*");

    if let Ok(url) = reqwest::Url::parse(&subscription.url) {
        if let Some(host) = url.host_str() {
            let referer = format!("{}://{}/", url.scheme(), host);
            builder = builder.header("Referer", referer);
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

pub async fn add_subscription(name: String, url: String, user_agent: Option<String>) -> Result<()> {
    let mut storage = load_subscriptions().await.unwrap_or_default();
    
    let subscription = Subscription {
        id: Uuid::new_v4().to_string(),
        name,
        url,
        user_agent,
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
        
        // Fetch subscription content
        match fetch_subscription_content(&subscription_clone).await {
            Ok(proxy_count) => {
                if let Some(sub) = storage.subscriptions.get_mut(id) {
                    sub.status = SubscriptionStatus::Active;
                    sub.proxy_count = proxy_count;
                    sub.last_updated = chrono::Utc::now().to_rfc3339();
                    sub.last_error = None;
                }

                // Regenerate config with proxies from this subscription
                if let Err(err) = generate_config_from_subscriptions(vec![id.to_string()]).await {
                    if let Some(sub) = storage.subscriptions.get_mut(id) {
                        sub.status = SubscriptionStatus::Error;
                        sub.last_error = Some(format!("生成配置失败: {}", err));
                    }
                } else if let Err(err) = crate::mihomo::reload_config().await {
                    if let Some(sub) = storage.subscriptions.get_mut(id) {
                        sub.status = SubscriptionStatus::Error;
                        sub.last_error = Some(format!("配置重载失败: {}", err));
                    }
                }
            }
            Err(e) => {
                if let Some(sub) = storage.subscriptions.get_mut(id) {
                    sub.status = SubscriptionStatus::Error;
                    sub.last_error = Some(e.to_string());
                }
            }
        }
        
        save_subscriptions(&storage).await?;
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
                proxy_names.push(proxy["name"].as_str().unwrap_or("").to_string());
                all_proxies.push(proxy);
            }
        }
    }
    
    // Generate config with proxies
    let mut config = crate::config::load_config().await?;
    config["proxies"] = serde_json::json!(all_proxies);
    
    // Create or update proxy groups
    let mut proxy_list = vec!["auto".to_string()];
    proxy_list.extend(proxy_names.clone());
    
    let proxy_groups = vec![
        serde_json::json!({
            "name": "PROXY",
            "type": "select",
            "proxies": proxy_list
        }),
        serde_json::json!({
            "name": "auto",
            "type": "url-test",
            "proxies": proxy_names.clone(),
            "url": "http://www.gstatic.com/generate_204",
            "interval": 300
        })
    ];
    
    config["proxy-groups"] = serde_json::json!(proxy_groups);
    
    crate::config::save_config(config).await?;
    
    Ok(())
}

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
    parse_subscription_content(&content)
}

fn parse_subscription_content(content: &str) -> Result<Vec<serde_json::Value>> {
    // Try to decode as base64 first
    let decoded_content = if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content) {
        String::from_utf8(decoded).unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    };
    
    // Parse as YAML
    let docs = yaml_rust::YamlLoader::load_from_str(&decoded_content)
        .context("Failed to parse subscription YAML")?;
    
    if let Some(doc) = docs.get(0) {
        if let Some(proxies) = doc["proxies"].as_vec() {
            let mut result = Vec::new();
            for proxy in proxies {
                if let Ok(json_proxy) = crate::config::yaml_to_json(proxy) {
                    result.push(json_proxy);
                }
            }
            return Ok(result);
        }
    }
    
    // If YAML parsing fails, try parsing individual proxy URLs
    let lines: Vec<&str> = decoded_content.lines().collect();
    let mut proxies = Vec::new();
    
    for line in lines {
        if let Ok(proxy) = parse_proxy_url(line) {
            proxies.push(proxy);
        }
    }
    
    if proxies.is_empty() {
        return Err(anyhow::anyhow!("No valid proxies found in subscription"));
    }
    
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
    let url = url.strip_prefix("ss://").unwrap();
    
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
