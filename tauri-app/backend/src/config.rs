use anyhow::{Context, Result};
pub use mihomoapp::yaml::yaml_to_json;
use std::fs;
use std::path::PathBuf;

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

/// Save an internally generated candidate only after structural validation.
pub async fn save_validated_config(config: serde_json::Value) -> Result<()> {
    let manager = crate::config_manager::get_config_manager().await?;
    manager.write_validated_config(config).await
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

    let storage = crate::subscription::get_subscriptions()
        .await
        .unwrap_or_default();
    let active_subscriptions: Vec<_> = storage
        .iter()
        .filter(|s| {
            matches!(s.status, crate::subscription::SubscriptionStatus::Active) && s.proxy_count > 0
        })
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

    let subscription_ids: Vec<String> = active_subscriptions.iter().map(|s| s.id.clone()).collect();

    match crate::subscription::generate_config_from_subscriptions(subscription_ids).await {
        Ok(_) => {
            tracing::info!("配置已重置为默认值并重新生成（使用本地缓存的订阅）");
            Ok("配置已重置为默认值，并使用本地缓存的订阅重新生成了运行时配置。".to_string())
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

            Ok(format!(
                "配置已重置为默认值，但重新生成订阅配置失败: {}。请手动更新订阅。",
                e
            ))
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

    tracing::info!(
        "检测到旧配置版本 {}，升级到版本 {}",
        current_version,
        CONFIG_VERSION
    );

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
            if let Some(pos) = rules
                .iter()
                .position(|r| r.as_str().map(|s| s.starts_with("GEOIP,")).unwrap_or(false))
            {
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
        "mihomo-manager-default-client-fingerprint": "chrome",
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

/// Called only for an explicit core/service start. Ordinary reads do not migrate
/// user files. A changed candidate must pass the selected core's isolated -t test.
pub async fn prepare_config_for_core(core_binary: &std::path::Path) -> Result<()> {
    let config_path = get_config_path()?;
    let manager = crate::config_manager::get_config_manager().await?;
    prepare_config_file_for_core(&manager, &config_path, core_binary).await
}

async fn prepare_config_file_for_core(
    manager: &crate::config_manager::ConfigManager,
    config_path: &std::path::Path,
    core_binary: &std::path::Path,
) -> Result<()> {
    let original = fs::read(config_path).context("无法读取待兼容配置")?;
    let content = std::str::from_utf8(&original).context("配置不是 UTF-8 文本")?;
    let docs = yaml_rust::YamlLoader::load_from_str(content).context("无法解析待兼容配置")?;
    let doc = docs.first().context("待兼容配置为空")?;
    let mut candidate = yaml_to_json(doc).context("无法转换待兼容配置")?;
    let had_legacy = candidate.get("global-client-fingerprint").is_some();
    let rewrite = mihomoapp::config_compat::normalize_runtime_config(&mut candidate)?;
    let provider_fallback = candidate
        .get(mihomoapp::config_compat::DEFAULT_FINGERPRINT_KEY)
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty())
        && candidate
            .get("proxy-providers")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|providers| !providers.is_empty());
    if !rewrite && !provider_fallback {
        return Ok(());
    }
    let version = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::process::Command::new(core_binary)
            .arg("-v")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("核心版本检查超时，当前配置未修改")?
    .context("无法执行核心版本检查，当前配置未修改")?;
    if !version.status.success() {
        return Err(anyhow::anyhow!("核心版本检查失败，当前配置未修改"));
    }
    let version =
        std::str::from_utf8(&version.stdout).context("核心版本输出无效，当前配置未修改")?;
    if !mihomoapp::config_compat::core_requires_migration(version)? {
        if had_legacy {
            return Ok(());
        }
        if provider_fallback {
            return Err(anyhow::anyhow!("所选旧核心无法保证应用代理集指纹规则，需使用项目 v1.19.33 或更新内核；当前配置未修改"));
        }
        return Ok(());
    }
    if provider_fallback
        && !mihomoapp::config_compat::core_supports_provider_compatibility(version)?
    {
        return Err(anyhow::anyhow!(
            "代理集指纹兼容规则已针对 v1.19.33 验证，请使用项目内核；当前配置未修改"
        ));
    }
    if !rewrite {
        return Ok(());
    }
    let source_dir = config_path.parent().context("无法定位配置目录")?;
    let staged = CompatibilityValidationDir::new(source_dir, &candidate)?;
    let candidate_path = staged.0.join("config.yaml");
    let yaml = serde_yaml::to_string(&candidate).context("无法序列化兼容候选")?;
    fs::write(&candidate_path, yaml).context("无法写入独立兼容候选")?;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new(core_binary)
            .arg("-t")
            .arg("-d")
            .arg(&staged.0)
            .arg("-f")
            .arg(&candidate_path)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("新核心兼容校验超时，当前配置未修改")?
    .context("无法执行新核心兼容校验，当前配置未修改")?;
    if !output.status.success() {
        // Core diagnostics may include subscription URLs or node details. Do not
        // surface captured output through logs/UI errors.
        return Err(anyhow::anyhow!("新核心 -t 拒绝了兼容候选，当前配置未修改；请先修复原配置（例如规则引用不存在的代理组）"));
    }
    manager
        .write_config_if_unchanged(candidate, &original)
        .await
}

struct CompatibilityValidationDir(PathBuf);

impl Drop for CompatibilityValidationDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl CompatibilityValidationDir {
    fn new(source: &std::path::Path, config: &serde_json::Value) -> Result<Self> {
        let target =
            std::env::temp_dir().join(format!("mihomo-config-compat-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&target).context("无法创建独立兼容校验目录")?;
        let staged = Self(target);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged.0, fs::Permissions::from_mode(0o700))?;
        }
        for name in [
            "GeoSite.dat",
            "GeoIP.dat",
            "Country.mmdb",
            "ASN.mmdb",
            "geoip.metadb",
        ] {
            staged.copy_relative(source, std::path::Path::new(name))?;
        }
        for field in ["proxy-providers", "rule-providers"] {
            if let Some(providers) = config.get(field) {
                for provider in providers
                    .as_object()
                    .context("代理集/规则集必须为映射")?
                    .values()
                {
                    if let Some(path) = provider.get("path") {
                        let path = path.as_str().context("代理集/规则集 path 必须为字符串")?;
                        // Empty is Mihomo's automatic cache location. Let the
                        // selected core create that cache inside the staging -d.
                        if path.is_empty() {
                            continue;
                        }
                        staged.copy_relative(source, std::path::Path::new(path))?;
                    }
                }
            }
        }
        Ok(staged)
    }

    fn copy_relative(&self, source: &std::path::Path, relative: &std::path::Path) -> Result<()> {
        use std::path::Component;
        if relative.as_os_str().is_empty()
            || relative.to_string_lossy().contains(['\\', ':'])
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        {
            return Err(anyhow::anyhow!(
                "兼容校验不支持指向目录外的代理集/规则集路径，当前配置未修改"
            ));
        }
        let from = source.join(relative);
        if !from.exists() {
            return Ok(());
        }
        let canonical_source = source.canonicalize().context("无法定位原配置资源目录")?;
        let canonical_file = from.canonicalize().context("无法定位原配置资源")?;
        if !canonical_file.starts_with(&canonical_source) || !canonical_file.is_file() {
            return Err(anyhow::anyhow!(
                "兼容校验资源超出原配置目录或不是文件，当前配置未修改"
            ));
        }
        let to = self.0.join(relative);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&canonical_file, to).context("无法复制独立校验所需资源")?;
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod compatibility_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("mihomo-compat-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn core(&self, body: &str) -> PathBuf {
            let core = self.0.join("fake-core");
            fs::write(
                &core,
                format!(
                    r#"#!/bin/sh
if [ "$1" = "-v" ]; then echo 'Mihomo Meta v1.19.33 linux amd64'; exit 0; fi
{}
"#,
                    body
                ),
            )
            .unwrap();
            fs::set_permissions(&core, fs::Permissions::from_mode(0o755)).unwrap();
            core
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn unchanged_configuration_never_executes_core_or_rewrites_bytes() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original = b"# keep comment\nmode: direct\nproxies: []\n";
        fs::write(&path, original).unwrap();
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        prepare_config_file_for_core(&manager, &path, &dir.0.join("absent-core"))
            .await
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!dir.0.join("backups").exists());
    }

    #[tokio::test]
    async fn older_selected_core_keeps_its_legacy_configuration_unchanged() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original = b"global-client-fingerprint: chrome\nproxies: []\n";
        fs::write(&path, original).unwrap();
        let core = dir.core("exit 9");
        let script = fs::read_to_string(&core)
            .unwrap()
            .replace("v1.19.33", "v1.19.19");
        fs::write(&core, script).unwrap();
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        prepare_config_file_for_core(&manager, &path, &core)
            .await
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!dir.0.join("backups").exists());
    }

    #[tokio::test]
    async fn provider_migration_is_refused_for_an_unverified_older_core() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original = b"global-client-fingerprint: chrome\nproxy-providers:\n  p:\n    type: inline\n    payload: []\n";
        fs::write(&path, original).unwrap();
        let core = dir.core("exit 0");
        let script = fs::read_to_string(&core)
            .unwrap()
            .replace("v1.19.33", "v1.19.27");
        fs::write(&core, script).unwrap();
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        assert!(prepare_config_file_for_core(&manager, &path, &core)
            .await
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!dir.0.join("backups").exists());
    }

    #[tokio::test]
    async fn rejected_candidate_keeps_original_configuration_and_assets() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original = b"# user config\nglobal-client-fingerprint: chrome\nproxies: []\n";
        fs::write(&path, original).unwrap();
        fs::write(dir.0.join("GeoSite.dat"), "original asset").unwrap();
        let core = dir.core("exit 7");
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        assert!(prepare_config_file_for_core(&manager, &path, &core)
            .await
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(
            fs::read_to_string(dir.0.join("GeoSite.dat")).unwrap(),
            "original asset"
        );
        assert!(!dir.0.join("backups").exists());
    }

    #[tokio::test]
    async fn accepted_candidate_is_isolated_and_has_a_recoverable_backup() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original =
            b"global-client-fingerprint: chrome\nproxies:\n- type: vless\n  name: node\n";
        fs::write(&path, original).unwrap();
        fs::write(dir.0.join("GeoSite.dat"), "original asset").unwrap();
        // A core writes only to its supplied -d directory. Its marker must not
        // appear alongside the user's original file.
        let core = dir.core(
            r#"test "$1" = "-t" || exit 8
printf changed > "$3/GeoSite.dat"
touch "$3/test-marker"
exit 0"#,
        );
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        prepare_config_file_for_core(&manager, &path, &core)
            .await
            .unwrap();
        assert!(!dir.0.join("test-marker").exists());
        assert_eq!(
            fs::read_to_string(dir.0.join("GeoSite.dat")).unwrap(),
            "original asset"
        );
        let config = manager.read_config().await.unwrap();
        assert_eq!(config["proxies"][0]["client-fingerprint"], "chrome");
        let backups: Vec<_> = fs::read_dir(dir.0.join("backups"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), original);
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[tokio::test]
    async fn bundled_core_validates_and_migrates_a_real_legacy_configuration() {
        let core = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/mihomo");
        if !core.is_file() {
            return;
        }
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let original = br#"mixed-port: 17890
external-controller: 127.0.0.1:19731
log-level: error
global-client-fingerprint: chrome
tun: {enable: false}
dns: {enable: false}
proxies:
  - {name: node, type: vless, server: example.invalid, port: 443, uuid: 00000000-0000-0000-0000-000000000001, tls: true}
proxy-providers:
  example:
    type: inline
    payload:
      - {name: provider-node, type: vless, server: example.invalid, port: 443, uuid: 00000000-0000-0000-0000-000000000001, tls: true}
proxy-groups:
  - {name: PROXY, type: select, proxies: [node], use: [example]}
rules: ["MATCH,PROXY"]
"#;
        fs::write(&path, original).unwrap();
        let manager = crate::config_manager::ConfigManager::new(path.clone());
        prepare_config_file_for_core(&manager, &path, &core)
            .await
            .unwrap();
        let result = manager.read_config().await.unwrap();
        assert!(result.get("global-client-fingerprint").is_none());
        assert_eq!(result["proxies"][0]["client-fingerprint"], "chrome");
        assert!(result["proxy-providers"]["example"]["override"]["override-expr"].is_array());
        let backups: Vec<_> = fs::read_dir(dir.0.join("backups"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), original);
    }

    #[test]
    fn external_resource_paths_are_refused_before_a_candidate_can_run() {
        let dir = TestDirectory::new();
        for path in ["../external.yaml", "/external.yaml", "C:\\external.yaml"] {
            let config = serde_json::json!({"proxy-providers":{"p":{"path":path}}});
            assert!(CompatibilityValidationDir::new(&dir.0, &config).is_err());
        }
    }

    #[test]
    fn empty_provider_paths_use_isolated_automatic_cache_locations() {
        let dir = TestDirectory::new();
        let config = serde_json::json!({
            "proxy-providers": {"proxy": {"type": "http", "path": ""}},
            "rule-providers": {"rules": {"type": "http", "path": ""}}
        });
        let staged = CompatibilityValidationDir::new(&dir.0, &config).unwrap();
        assert_ne!(staged.0, dir.0);
        assert_eq!(fs::read_dir(&staged.0).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }
}
