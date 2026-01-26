use anyhow::Result;
use std::collections::HashSet;

/// 配置验证结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// 验证mihomo配置文件
pub async fn validate_config(config: &serde_json::Value) -> Result<ValidationResult> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. 验证必需字段
    validate_required_fields(config, &mut errors);

    // 2. 验证端口配置
    validate_ports(config, &mut errors, &mut warnings);

    // 3. 验证DNS配置
    validate_dns(config, &mut warnings);

    // 4. 验证代理节点
    validate_proxies(config, &mut errors, &mut warnings);

    // 5. 验证代理组
    validate_proxy_groups(config, &mut errors, &mut warnings);

    // 6. 验证规则
    validate_rules(config, &mut warnings);

    let valid = errors.is_empty();

    Ok(ValidationResult {
        valid,
        errors,
        warnings,
    })
}

fn validate_required_fields(config: &serde_json::Value, errors: &mut Vec<String>) {
    let required_fields = vec!["port", "socks-port", "external-controller"];

    for field in required_fields {
        if config.get(field).is_none() {
            errors.push(format!("缺少必需字段: {}", field));
        }
    }
}

fn validate_ports(
    config: &serde_json::Value,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let port_fields = vec!["port", "socks-port", "mixed-port"];
    let mut used_ports = HashSet::new();

    for field in port_fields {
        if let Some(port) = config.get(field).and_then(|v| v.as_u64()) {
            // 检查端口范围
            if port == 0 || port > 65535 {
                errors.push(format!("{} 端口无效: {}", field, port));
            }

            // 检查端口冲突
            if used_ports.contains(&port) {
                warnings.push(format!("端口 {} 被多个字段使用", port));
            }
            used_ports.insert(port);

            // 检查常用端口
            if port < 1024 {
                warnings.push(format!(
                    "{} 使用了系统保留端口 {}, 可能需要管理员权限",
                    field, port
                ));
            }
        }
    }
}

fn validate_dns(config: &serde_json::Value, warnings: &mut Vec<String>) {
    if let Some(dns) = config.get("dns") {
        if let Some(enable) = dns.get("enable").and_then(|v| v.as_bool()) {
            if !enable {
                warnings.push("DNS未启用，可能影响域名解析速度".to_string());
            }
        }

        // 检查DNS服务器配置
        if let Some(nameservers) = dns.get("nameserver").and_then(|v| v.as_array()) {
            if nameservers.is_empty() {
                warnings.push("未配置DNS服务器".to_string());
            }
        }
    } else {
        warnings.push("未配置DNS，建议启用DNS以提升性能".to_string());
    }
}

fn validate_proxies(
    config: &serde_json::Value,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if let Some(proxies) = config.get("proxies").and_then(|v| v.as_array()) {
        if proxies.is_empty() {
            warnings.push("没有配置代理节点".to_string());
            return;
        }

        let mut proxy_names = HashSet::new();

        for (idx, proxy) in proxies.iter().enumerate() {
            // 检查必需字段
            if proxy.get("name").is_none() {
                errors.push(format!("代理节点 #{} 缺少name字段", idx));
            }

            if proxy.get("type").is_none() {
                errors.push(format!("代理节点 #{} 缺少type字段", idx));
            }

            if proxy.get("server").is_none() {
                errors.push(format!("代理节点 #{} 缺少server字段", idx));
            }

            // 检查名称重复
            if let Some(name) = proxy.get("name").and_then(|v| v.as_str()) {
                if proxy_names.contains(name) {
                    errors.push(format!("代理节点名称重复: {}", name));
                }
                proxy_names.insert(name.to_string());
            }

            // 检查代理类型
            if let Some(proxy_type) = proxy.get("type").and_then(|v| v.as_str()) {
                let valid_types = [
                    "ss", "ssr", "vmess", "vless", "trojan", "snell", "http", "socks5",
                ];
                if !valid_types.contains(&proxy_type) {
                    warnings.push(format!(
                        "代理节点 #{} 使用了不常见的类型: {}",
                        idx, proxy_type
                    ));
                }
            }
        }
    } else {
        warnings.push("没有配置代理节点".to_string());
    }
}

fn validate_proxy_groups(
    config: &serde_json::Value,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if let Some(groups) = config.get("proxy-groups").and_then(|v| v.as_array()) {
        if groups.is_empty() {
            warnings.push("没有配置代理组".to_string());
            return;
        }

        let mut group_names = HashSet::new();
        let proxy_names: HashSet<String> =
            if let Some(proxies) = config.get("proxies").and_then(|v| v.as_array()) {
                proxies
                    .iter()
                    .filter_map(|p| {
                        p.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            } else {
                HashSet::new()
            };

        for (idx, group) in groups.iter().enumerate() {
            // 检查必需字段
            if group.get("name").is_none() {
                errors.push(format!("代理组 #{} 缺少name字段", idx));
            }

            if group.get("type").is_none() {
                errors.push(format!("代理组 #{} 缺少type字段", idx));
            }

            if group.get("proxies").is_none() {
                errors.push(format!("代理组 #{} 缺少proxies字段", idx));
            }

            // 检查名称重复
            if let Some(name) = group.get("name").and_then(|v| v.as_str()) {
                if group_names.contains(name) {
                    errors.push(format!("代理组名称重复: {}", name));
                }
                group_names.insert(name.to_string());
            }

            // 检查代理组类型
            if let Some(group_type) = group.get("type").and_then(|v| v.as_str()) {
                let valid_types = ["select", "url-test", "fallback", "load-balance", "relay"];
                if !valid_types.contains(&group_type) {
                    errors.push(format!("代理组 #{} 使用了无效的类型: {}", idx, group_type));
                }
            }

            // 检查代理组引用的节点是否存在
            if let Some(proxies) = group.get("proxies").and_then(|v| v.as_array()) {
                for proxy_ref in proxies {
                    if let Some(proxy_name) = proxy_ref.as_str() {
                        // 跳过特殊节点
                        if proxy_name == "DIRECT" || proxy_name == "REJECT" {
                            continue;
                        }

                        // 检查是否是其他代理组
                        if group_names.contains(proxy_name) {
                            continue;
                        }

                        // 检查是否是实际的代理节点
                        if !proxy_names.contains(proxy_name) {
                            warnings.push(format!("代理组引用了不存在的节点: {}", proxy_name));
                        }
                    }
                }
            }
        }
    } else {
        warnings.push("没有配置代理组".to_string());
    }
}

fn validate_rules(config: &serde_json::Value, warnings: &mut Vec<String>) {
    if let Some(rules) = config.get("rules").and_then(|v| v.as_array()) {
        if rules.is_empty() {
            warnings.push("没有配置路由规则".to_string());
            return;
        }

        // 检查是否有MATCH规则
        let has_match = rules.iter().any(|r| {
            if let Some(rule_str) = r.as_str() {
                rule_str.starts_with("MATCH,")
            } else {
                false
            }
        });

        if !has_match {
            warnings.push("建议添加MATCH规则作为默认规则".to_string());
        }
    } else {
        warnings.push("没有配置路由规则".to_string());
    }
}
