use anyhow::{Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::RwLock;

/// A successful API connection is retained independently of edits to the file.
/// This is connection state only: it never establishes process/service ownership.
struct ConnectionState {
    settings: Option<ControllerSettings>,
    generation: u64,
}

#[cfg(test)]
pub static TEST_CONNECTION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

static ACTIVE_CONNECTION: RwLock<ConnectionState> = RwLock::const_new(ConnectionState {
    settings: None,
    generation: 0,
});

#[derive(Clone, PartialEq, Eq)]
struct ControllerSettings {
    base: Url,
    secret: String,
}

impl ControllerSettings {
    fn from_config(config: &Value) -> Result<Self> {
        let address = match config.get("external-controller") {
            None => "127.0.0.1:9090",
            Some(value) => value
                .as_str()
                .context("external-controller 必须为地址字符串")?,
        };
        if address.trim().is_empty() {
            return Err(anyhow::anyhow!("配置未启用 HTTP 控制器，无法连接核心 API"));
        }
        let address = if address.starts_with(':') {
            format!("127.0.0.1{}", address)
        } else {
            address.to_owned()
        };
        if !address
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
            .is_some_and(|port| port > 0)
        {
            return Err(anyhow::anyhow!("控制器端口必须介于 1 和 65535"));
        }
        let mut base = Url::parse(&format!("http://{}", address))
            .context("控制器地址无效，应为 host:port 或 [IPv6]:port")?;
        if base.host_str().is_none()
            || base.port_or_known_default().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(anyhow::anyhow!(
                "控制器地址无效，应为 host:port 或 [IPv6]:port"
            ));
        }
        match base.host_str() {
            Some("0.0.0.0") => base.set_host(Some("127.0.0.1")).context("控制器地址无效")?,
            Some("[::]") => base.set_host(Some("[::1]")).context("控制器地址无效")?,
            _ => {}
        }
        let secret = match config.get("secret") {
            None | Some(Value::Null) => String::new(),
            Some(value) => value
                .as_str()
                .context("控制器 secret 必须为字符串")?
                .to_owned(),
        };
        Ok(Self { base, secret })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("控制器地址不能构造 API 路径"))?
            .clear()
            .extend(segments);
        Ok(url)
    }
}

pub struct ControllerClient {
    settings: ControllerSettings,
    client: Client,
}

impl ControllerClient {
    fn new(settings: ControllerSettings, timeout: Duration) -> Result<Self> {
        Ok(Self {
            settings,
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(2))
                .timeout(timeout)
                .build()
                .context("创建核心 API 客户端失败")?,
        })
    }

    pub async fn active(timeout: Duration) -> Result<Self> {
        let settings = match ACTIVE_CONNECTION.read().await.settings.clone() {
            Some(settings) => settings,
            None => configured_settings().await?,
        };
        Self::new(settings, timeout)
    }

    pub fn request(&self, method: Method, segments: &[&str]) -> Result<RequestBuilder> {
        let mut request = self
            .client
            .request(method, self.settings.endpoint(segments)?);
        if !self.settings.secret.is_empty() {
            request = request.bearer_auth(&self.settings.secret);
        }
        Ok(request)
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response> {
        let response = request
            .send()
            .await
            .map_err(|error| error.without_url())
            .context("无法连接核心控制器，请检查控制器地址、核心运行状态及待应用配置")?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(anyhow::anyhow!(
                "核心控制器认证失败（HTTP {}），请检查 secret 及配置是否已应用",
                status.as_u16()
            ));
        }
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "核心控制器请求失败（HTTP {}）",
                status.as_u16()
            ));
        }
        Ok(response)
    }

    pub async fn get_json(&self, segments: &[&str]) -> Result<Value> {
        self.send(self.request(Method::GET, segments)?)
            .await?
            .json()
            .await
            .map_err(|error| error.without_url())
            .context("核心控制器返回了无效 JSON")
    }
}

async fn configured_settings() -> Result<ControllerSettings> {
    // read_config is deliberately used instead of load_config, which may create,
    // upgrade and save a file while merely checking the controller.
    let manager = crate::config_manager::get_config_manager().await?;
    ControllerSettings::from_config(&manager.read_config().await?)
}

async fn verify_connection(client: ControllerClient, generation: u64, replace: bool) -> Result<()> {
    let version = client.get_json(&["version"]).await?;
    if !version
        .get("version")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
    {
        return Err(anyhow::anyhow!(
            "控制器未返回有效的 Mihomo 版本，无法确认 API 连接"
        ));
    }
    // A stop or a newer explicit connection must not be undone by an old probe.
    let mut active = ACTIVE_CONNECTION.write().await;
    if active.generation != generation {
        return Err(anyhow::anyhow!("核心连接状态已变化，请重试"));
    }
    // This identifies a usable API, never process/service ownership.
    if replace {
        active.generation = active.generation.wrapping_add(1);
        active.settings = Some(client.settings);
    } else if active.settings.is_none() {
        active.settings = Some(client.settings);
    }
    Ok(())
}

pub async fn health() -> Result<()> {
    let generation = ACTIVE_CONNECTION.read().await.generation;
    verify_connection(
        ControllerClient::active(Duration::from_secs(2)).await?,
        generation,
        false,
    )
    .await
}

/// Call after an explicit start/restart. A failed candidate never replaces the
/// previous working connection, which may still be serving the existing core.
pub async fn connect_configured() -> Result<()> {
    let generation = ACTIVE_CONNECTION.read().await.generation;
    verify_connection(
        ControllerClient::new(configured_settings().await?, Duration::from_secs(2))?,
        generation,
        true,
    )
    .await
}

pub async fn clear_connection() {
    let mut active = ACTIVE_CONNECTION.write().await;
    active.settings = None;
    active.generation = active.generation.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn controller_addresses_normalize_wildcards_and_encode_path_segments() {
        for (address, host) in [
            ("0.0.0.0:9091", "127.0.0.1"),
            ("[::]:9091", "[::1]"),
            (":9091", "127.0.0.1"),
        ] {
            let settings =
                ControllerSettings::from_config(&json!({"external-controller": address})).unwrap();
            assert_eq!(settings.base.host_str(), Some(host));
            let endpoint = settings
                .endpoint(&["proxies", "香港/自动?x#1", "delay"])
                .unwrap();
            assert!(endpoint.as_str().contains("%2F"));
            assert!(endpoint.query().is_none());
            assert!(endpoint.fragment().is_none());
            assert_eq!(endpoint.path_segments().unwrap().count(), 3);
        }
        for invalid in [
            "",
            "http://127.0.0.1:9090",
            "host:9090/path",
            "user@host:9090",
            "host:9090?x=1",
        ] {
            assert!(
                ControllerSettings::from_config(&json!({"external-controller": invalid})).is_err()
            );
        }
    }

    #[tokio::test]
    async fn requests_use_configured_secret_and_reject_http_errors_without_disclosing_it() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let count = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..count]).to_lowercase();
            assert!(request.contains("authorization: bearer test-secret"));
            assert!(request.contains(
                "get /proxies/group%2fname%3fvalue%23 HTTP/1.1"
                    .to_lowercase()
                    .as_str()
            ));
            socket.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
        });
        let settings = ControllerSettings::from_config(
            &json!({"external-controller": address.to_string(), "secret": "test-secret"}),
        )
        .unwrap();
        let client = ControllerClient::new(settings, Duration::from_secs(2)).unwrap();
        let error = client
            .get_json(&["proxies", "group/name?value#"])
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("401"));
        assert!(!error.contains("test-secret"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn active_connection_keeps_the_verified_runtime_snapshot() {
        let _test_guard = TEST_CONNECTION_LOCK.lock().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            socket.read(&mut request).await.unwrap();
            let body = r#"{"version":"test-version"}"#;
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
        });
        let settings = ControllerSettings::from_config(
            &json!({"external-controller": address.to_string(), "secret": "runtime-secret"}),
        )
        .unwrap();
        let generation = ACTIVE_CONNECTION.read().await.generation;
        verify_connection(
            ControllerClient::new(settings.clone(), Duration::from_secs(2)).unwrap(),
            generation,
            true,
        )
        .await
        .unwrap();
        // Active API calls reuse the verified endpoint instead of reading edited disk settings.
        let active = ControllerClient::active(Duration::from_secs(2))
            .await
            .unwrap();
        assert!(active.settings == settings);
        clear_connection().await;
        server.await.unwrap();
    }
}
