use anyhow::{Context, Result};
use serde_json::Value;

/// Provider identity stays separate from the display name. Mihomo's selector API
/// accepts only names, so duplicate names cannot be disambiguated by a dashboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyTarget {
    pub name: String,
    pub provider: Option<String>,
}

impl ProxyTarget {
    pub fn delay_path(&self) -> Vec<&str> {
        match self.provider.as_deref() {
            Some(provider) => vec!["providers", "proxies", provider, &self.name, "healthcheck"],
            None => vec!["proxies", &self.name, "delay"],
        }
    }
}

pub fn is_node(proxy: &Value) -> bool {
    proxy
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| {
            ![
                "Selector",
                "URLTest",
                "Fallback",
                "LoadBalance",
                "Relay",
                "Direct",
                "Reject",
                "RejectDrop",
                "Compatible",
                "Pass",
                "PassRule",
            ]
            .contains(&kind)
        })
}

pub fn test_targets(response: &Value) -> Result<Vec<ProxyTarget>> {
    let proxies = response["proxies"]
        .as_object()
        .context("Invalid proxies response")?;
    let providers = response["providers"]
        .as_object()
        .context("Invalid providers response")?;
    let mut targets = Vec::new();
    for (name, proxy) in proxies {
        // Older Mihomo releases also flattened provider nodes into /proxies.
        // Their explicit source is authoritative; do not test them twice.
        let external_provider = proxy["provider-name"]
            .as_str()
            .filter(|name| !name.is_empty());
        if is_node(proxy) && external_provider.is_none() {
            targets.push(ProxyTarget {
                name: name.clone(),
                provider: None,
            });
        }
    }
    for (provider, data) in providers {
        if data["vehicleType"].as_str() == Some("Compatible") {
            continue;
        }
        let nodes = data["proxies"]
            .as_array()
            .context("Invalid provider proxy list")?;
        for node in nodes {
            if !is_node(node) {
                continue;
            }
            let name = node["name"]
                .as_str()
                .context("Provider proxy is missing its name")?;
            let target = ProxyTarget {
                name: name.to_owned(),
                provider: Some(provider.clone()),
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
        }
    }
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn provider_names_stay_scoped_and_compatible_wrappers_are_not_retested() {
        let response = json!({
            "proxies": {"same": {"type": "Vless"}, "PROXY": {"type": "Selector"}, "DIRECT": {"type": "Direct"}},
            "providers": {
                "default": {"vehicleType": "Compatible", "proxies": [{"name": "same", "type": "Vless"}]},
                "a": {"vehicleType": "HTTP", "proxies": [{"name": "same", "type": "Trojan"}]},
                "b": {"vehicleType": "Inline", "proxies": [{"name": "same", "type": "Shadowsocks"}]}
            }
        });
        let targets = test_targets(&response).unwrap();
        assert_eq!(targets.len(), 3);
        assert!(targets.contains(&ProxyTarget {
            name: "same".into(),
            provider: None
        }));
        assert!(targets.contains(&ProxyTarget {
            name: "same".into(),
            provider: Some("a".into())
        }));
        assert!(targets.contains(&ProxyTarget {
            name: "same".into(),
            provider: Some("b".into())
        }));
        assert_eq!(
            targets
                .iter()
                .find(|target| target.provider.as_deref() == Some("a"))
                .unwrap()
                .delay_path(),
            ["providers", "proxies", "a", "same", "healthcheck"]
        );
    }

    #[test]
    fn old_flattened_provider_nodes_are_only_tested_at_the_provider_endpoint() {
        let response = json!({"proxies": {"node": {"type":"Vless", "provider-name":"a"}},
            "providers": {"a": {"vehicleType":"HTTP", "proxies":[{"name":"node", "type":"Vless"}]}}});
        assert_eq!(
            test_targets(&response).unwrap(),
            [ProxyTarget {
                name: "node".into(),
                provider: Some("a".into())
            }]
        );
        assert!(
            test_targets(&json!({"proxies":{},"providers":{"a":{"vehicleType":"HTTP"}}})).is_err()
        );
    }
}
