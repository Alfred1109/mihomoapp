//! Compatibility of application configuration with Mihomo v1.19.27 and newer.
//! The application-owned default is materialized into individual nodes. It is
//! retained as metadata so subsequent edits/imports do not lose that default.
use anyhow::{Context, Result};
use serde_json::Value;

pub const DEFAULT_FINGERPRINT_KEY: &str = "mihomo-manager-default-client-fingerprint";
const LEGACY_FINGERPRINT_KEY: &str = "global-client-fingerprint";

/// Version-gate runtime migration so an older explicitly selected core can keep
/// reading its original global setting rather than receiving newer provider syntax.
fn core_version(reported: &str) -> Result<(u32, u32, u32)> {
    let mut words = reported.split_whitespace();
    if words.next() != Some("Mihomo") || words.next() != Some("Meta") {
        return Err(anyhow::anyhow!("无法确认所选核心版本，当前配置未修改"));
    }
    let version = words
        .next()
        .and_then(|value| value.strip_prefix('v'))
        .context("无法确认所选核心版本，当前配置未修改")?;
    let parts = version
        .split('.')
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("无法确认所选核心版本，当前配置未修改")?;
    if parts.len() != 3 {
        return Err(anyhow::anyhow!("无法确认所选核心版本，当前配置未修改"));
    }
    Ok((parts[0], parts[1], parts[2]))
}

pub fn core_requires_migration(reported: &str) -> Result<bool> {
    Ok(core_version(reported)? >= (1, 19, 27))
}

/// Provider conditional overrides are verified against the project's v1.19.33
/// core. Older selected cores must not silently ignore generated fallback logic.
pub fn core_supports_provider_compatibility(reported: &str) -> Result<bool> {
    Ok(core_version(reported)? >= (1, 19, 33))
}

fn fingerprint(value: &Value, key: &str) -> Result<String> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(value) => Ok(value.clone()),
        _ => Err(anyhow::anyhow!("{} 必须为字符串", key)),
    }
}

fn configured_default(config: &Value) -> Result<Option<String>> {
    let legacy = config
        .get(LEGACY_FINGERPRINT_KEY)
        .map(|value| fingerprint(value, LEGACY_FINGERPRINT_KEY))
        .transpose()?;
    let current = config
        .get(DEFAULT_FINGERPRINT_KEY)
        .map(|value| fingerprint(value, DEFAULT_FINGERPRINT_KEY))
        .transpose()?;
    if let (Some(legacy), Some(current)) = (&legacy, &current) {
        if legacy != current {
            return Err(anyhow::anyhow!(
                "旧全局指纹与应用默认指纹冲突，请先明确保留哪一个设置"
            ));
        }
    }
    Ok(legacy.or(current))
}

fn apply_to_proxy(proxy: &mut Value, default: &str) -> Result<()> {
    let proxy = proxy.as_object_mut().context("代理节点必须为映射")?;
    let kind = proxy.get("type").and_then(Value::as_str).unwrap_or("");
    let supported = matches!(
        kind,
        "vmess" | "vless" | "trojan" | "anytls" | "gost-relay" | "trusttunnel"
    ) || (kind == "ss"
        && proxy.get("plugin").and_then(Value::as_str) == Some("shadow-tls"));
    let is_vless = kind == "vless";
    if supported
        && (proxy.get("client-fingerprint").is_none()
            || proxy
                .get("client-fingerprint")
                .is_some_and(|value| value.is_null() || value.as_str() == Some("")))
    {
        proxy.insert(
            "client-fingerprint".into(),
            Value::String(default.to_owned()),
        );
    }
    if is_vless {
        if let Some(download) = proxy
            .get_mut("xhttp-opts")
            .and_then(|options| options.get_mut("download-settings"))
            .and_then(Value::as_object_mut)
        {
            // Absent/null inherits the parent; explicit empty used the old global.
            if download.get("client-fingerprint").and_then(Value::as_str) == Some("") {
                download.insert(
                    "client-fingerprint".into(),
                    Value::String(default.to_owned()),
                );
            }
        }
    }
    Ok(())
}

/// Migrate a candidate in memory. Validation failure leaves the original value
/// untouched. Callers persist only through the normal backup/atomic-write path.
pub fn normalize_runtime_config(config: &mut Value) -> Result<bool> {
    let Some(default) = configured_default(config)? else {
        return Ok(false);
    };
    let mut candidate = config.clone();
    let object = candidate.as_object_mut().context("配置根节点必须为映射")?;
    object.remove(LEGACY_FINGERPRINT_KEY);
    object.insert(
        DEFAULT_FINGERPRINT_KEY.into(),
        Value::String(default.clone()),
    );
    if !default.is_empty() {
        if let Some(proxies) = candidate.get_mut("proxies") {
            for proxy in proxies.as_array_mut().context("proxies 必须为数组")? {
                apply_to_proxy(proxy, &default)?;
            }
        }
        if let Some(providers) = candidate.get_mut("proxy-providers") {
            let fingerprint = serde_json::to_string(&default)?;
            let applicable = r#"(.type == "vmess" or .type == "vless" or .type == "trojan" or .type == "anytls" or .type == "gost-relay" or .type == "trusttunnel" or (.type == "ss" and .plugin == "shadow-tls"))"#;
            let fallbacks = [
                format!(
                    r#"(select({applicable} and (."client-fingerprint" == null or ."client-fingerprint" == "")) | ."client-fingerprint") = {fingerprint}"#
                ),
                format!(
                    r#"(select(.type == "vless" and ."xhttp-opts"."download-settings"."client-fingerprint" == "") | ."xhttp-opts"."download-settings"."client-fingerprint") = {fingerprint}"#
                ),
            ];
            for provider in providers
                .as_object_mut()
                .context("proxy-providers 必须为映射")?
                .values_mut()
            {
                let provider = provider.as_object_mut().context("代理集必须为映射")?;
                let overrides = provider
                    .entry("override")
                    .or_insert_with(|| serde_json::json!({}))
                    .as_object_mut()
                    .context("代理集 override 必须为映射")?;
                let expressions = overrides
                    .entry("override-expr")
                    .or_insert_with(|| serde_json::json!([]))
                    .as_array_mut()
                    .context("代理集 override-expr 必须为数组")?;
                if !expressions.iter().all(Value::is_string) {
                    return Err(anyhow::anyhow!("代理集 override-expr 必须仅包含字符串"));
                }
                for fallback in &fallbacks {
                    if !expressions
                        .iter()
                        .any(|value| value.as_str() == Some(fallback))
                    {
                        expressions.push(Value::String(fallback.clone()));
                    }
                }
            }
        }
    }
    let changed = candidate != *config;
    *config = candidate;
    Ok(changed)
}

/// Base configurations hold the application default until nodes are merged.
pub fn normalize_base_config(config: &mut Value) -> Result<bool> {
    let Some(default) = configured_default(config)? else {
        return Ok(false);
    };
    let mut candidate = config.clone();
    let object = candidate
        .as_object_mut()
        .context("基础配置根节点必须为映射")?;
    object.remove(LEGACY_FINGERPRINT_KEY);
    object.insert(DEFAULT_FINGERPRINT_KEY.into(), Value::String(default));
    let changed = candidate != *config;
    *config = candidate;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migration_is_gated_by_the_selected_core_version() {
        assert!(!core_requires_migration("Mihomo Meta v1.19.19 linux amd64").unwrap());
        assert!(!core_requires_migration("Mihomo Meta v1.19.26 linux amd64").unwrap());
        assert!(core_requires_migration("Mihomo Meta v1.19.27 linux amd64").unwrap());
        assert!(core_requires_migration("Mihomo Meta v1.19.33 linux amd64").unwrap());
        for invalid in [
            "another core v1.19.33",
            "Mihomo Meta alpha",
            "Mihomo Meta v1.19.33-alpha",
        ] {
            assert!(core_requires_migration(invalid).is_err());
        }
    }

    #[test]
    fn legacy_transport_callers_and_xhttp_download_override_keep_their_behavior() {
        let mut config = json!({"global-client-fingerprint":"chrome","proxies":[
            {"type":"gost-relay"},{"type":"trusttunnel"},{"type":"ss","plugin":"shadow-tls"},
            {"type":"ss","plugin":"restls"},{"type":"http"},{"type":"socks5"},{"type":"hysteria2"},
            {"type":"vless","client-fingerprint":"firefox","xhttp-opts":{"download-settings":{"client-fingerprint":""}}},
            {"type":"vless","client-fingerprint":"none","xhttp-opts":{"download-settings":{}}}
        ]});
        normalize_runtime_config(&mut config).unwrap();
        for i in 0..3 {
            assert_eq!(config["proxies"][i]["client-fingerprint"], "chrome");
        }
        for i in 3..7 {
            assert!(config["proxies"][i].get("client-fingerprint").is_none());
        }
        assert_eq!(config["proxies"][7]["client-fingerprint"], "firefox");
        assert_eq!(
            config["proxies"][7]["xhttp-opts"]["download-settings"]["client-fingerprint"],
            "chrome"
        );
        assert!(config["proxies"][8]["xhttp-opts"]["download-settings"]
            .get("client-fingerprint")
            .is_none());
    }

    #[test]
    fn legacy_default_fills_only_supported_nodes_without_overwriting_explicit_settings() {
        let mut config = json!({"global-client-fingerprint":"chrome", "proxies":[
            {"type":"vmess"}, {"type":"vless","client-fingerprint":""},
            {"type":"trojan","client-fingerprint":"firefox"},
            {"type":"anytls","client-fingerprint":"none"}, {"type":"ss"}
        ]});
        assert!(normalize_runtime_config(&mut config).unwrap());
        assert!(config.get(LEGACY_FINGERPRINT_KEY).is_none());
        assert_eq!(config["proxies"][0]["client-fingerprint"], "chrome");
        assert_eq!(config["proxies"][1]["client-fingerprint"], "chrome");
        assert_eq!(config["proxies"][2]["client-fingerprint"], "firefox");
        assert_eq!(config["proxies"][3]["client-fingerprint"], "none");
        assert!(config["proxies"][4].get("client-fingerprint").is_none());
        assert!(!normalize_runtime_config(&mut config).unwrap());
    }

    #[test]
    fn provider_fallback_is_appended_after_existing_expressions_and_is_idempotent() {
        let mut config = json!({"global-client-fingerprint":"chrome", "proxy-providers":{
            "remote":{"type":"http","override":{"udp":true,"override-expr":[".name += \" suffix\""]}}
        }});
        normalize_runtime_config(&mut config).unwrap();
        let expressions = config["proxy-providers"]["remote"]["override"]["override-expr"]
            .as_array()
            .unwrap();
        assert_eq!(expressions.len(), 3);
        assert_eq!(expressions[0], ".name += \" suffix\"");
        assert!(expressions[1].as_str().unwrap().contains("gost-relay"));
        assert_eq!(config["proxy-providers"]["remote"]["override"]["udp"], true);
        assert!(!normalize_runtime_config(&mut config).unwrap());
    }

    #[test]
    fn absent_legacy_setting_leaves_user_configuration_unchanged() {
        let mut config = json!({"proxies":[{"type":"vless"}],"secret":"unchanged"});
        let original = config.clone();
        assert!(!normalize_runtime_config(&mut config).unwrap());
        assert_eq!(config, original);
    }

    #[test]
    fn malformed_or_conflicting_settings_leave_candidate_untouched() {
        for mut config in [
            json!({"global-client-fingerprint":true}),
            json!({
                "global-client-fingerprint":"chrome", "mihomo-manager-default-client-fingerprint":"none"
            }),
            json!({"global-client-fingerprint":"chrome","proxy-providers":{"bad":{"override":false}}}),
        ] {
            let original = config.clone();
            assert!(normalize_runtime_config(&mut config).is_err());
            assert_eq!(config, original);
        }
    }
}
