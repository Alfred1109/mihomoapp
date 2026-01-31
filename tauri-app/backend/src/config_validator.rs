use anyhow::{Context, Result};
use serde_json::{Value as JsonValue};
use std::collections::HashMap;
use tracing::{info, warn};

/// 配置标准化器 - 确保跨平台一致性
pub struct ConfigStandardizer;

impl ConfigStandardizer {
    /// 标准化配置文件，确保跨平台一致性
    pub fn standardize_config(config: &mut JsonValue) -> Result<Vec<String>> {
        let mut changes = Vec::new();
        
        // 1. 标准化DNS配置
        Self::standardize_dns(config, &mut changes)?;
        
        // 2. 标准化性能配置
        Self::standardize_performance(config, &mut changes)?;
        
        // 3. 标准化路由规则
        Self::standardize_rules(config, &mut changes)?;
        
        // 4. 标准化代理组
        Self::standardize_proxy_groups(config, &mut changes)?;
        
        // 5. 标准化TUN配置
        Self::standardize_tun(config, &mut changes)?;
        
        // 6. 设置配置版本
        config["config_version"] = JsonValue::String("2.0".to_string());
        
        if !changes.is_empty() {
            info!("配置已标准化，变更数: {}", changes.len());
        }
        
        Ok(changes)
    }
    
    /// 标准化DNS配置 - 三层架构
    fn standardize_dns(config: &mut JsonValue, changes: &mut Vec<String>) -> Result<()> {
        if config.get("dns").is_none() {
            config["dns"] = serde_json::json!({});
        }
        
        let dns = config["dns"].as_object_mut().unwrap();
        
        // Layer 0: 基础配置
        if !dns.contains_key("enable") || !dns["enable"].as_bool().unwrap_or(false) {
            dns.insert("enable".to_string(), JsonValue::Bool(true));
            changes.push("✅ 启用DNS服务器".to_string());
        }
        
        // IPv6优化
        if dns.get("ipv6").and_then(|v| v.as_bool()).unwrap_or(true) {
            dns.insert("ipv6".to_string(), JsonValue::Bool(false));
            changes.push("🚀 禁用IPv6以提升解析速度".to_string());
        }
        
        // HTTP/3优化
        if !dns.contains_key("prefer-h3") {
            dns.insert("prefer-h3".to_string(), JsonValue::Bool(true));
            changes.push("⚡ 启用HTTP/3 DoH加速".to_string());
        }
        
        // Fake-IP模式
        if dns.get("enhanced-mode").and_then(|v| v.as_str()) != Some("fake-ip") {
            dns.insert("enhanced-mode".to_string(), JsonValue::String("fake-ip".to_string()));
            changes.push("🎯 设置Fake-IP增强模式".to_string());
        }
        
        // Layer 1: 快速初始DNS
        if !dns.contains_key("default-nameserver") {
            dns.insert("default-nameserver".to_string(), serde_json::json!([
                "223.5.5.5",
                "119.29.29.29"
            ]));
            changes.push("🏃‍♂️ 添加快速初始DNS服务器".to_string());
        }
        
        // Layer 2: 主要DoH DNS
        if !dns.contains_key("nameserver") {
            dns.insert("nameserver".to_string(), serde_json::json!([
                "https://doh.pub/dns-query",
                "https://dns.alidns.com/dns-query"
            ]));
            changes.push("🔐 设置主要DoH DNS服务器".to_string());
        }
        
        // Layer 3: 防污染备用DNS
        if !dns.contains_key("fallback") {
            dns.insert("fallback".to_string(), serde_json::json!([
                "https://1.1.1.1/dns-query",
                "https://dns.google/dns-query"
            ]));
            changes.push("🛡️ 设置防污染备用DNS".to_string());
        }
        
        // 智能分流策略
        if !dns.contains_key("nameserver-policy") {
            dns.insert("nameserver-policy".to_string(), serde_json::json!({
                "geosite:cn,private,apple": [
                    "https://doh.pub/dns-query",
                    "https://dns.alidns.com/dns-query"
                ],
                "geosite:geolocation-!cn": [
                    "https://1.1.1.1/dns-query",
                    "https://dns.google/dns-query"
                ],
                "geosite:category-ads-all": "rcode://success"
            }));
            changes.push("🎯 配置DNS智能分流策略".to_string());
        }
        
        // Fallback过滤器
        if !dns.contains_key("fallback-filter") {
            dns.insert("fallback-filter".to_string(), serde_json::json!({
                "geoip": true,
                "geoip-code": "CN",
                "ipcidr": ["240.0.0.0/4"]
            }));
            changes.push("🧩 设置智能Fallback过滤器".to_string());
        }
        
        // Fake-IP排除列表
        if !dns.contains_key("fake-ip-filter") {
            dns.insert("fake-ip-filter".to_string(), serde_json::json!([
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
                "+.battle.net",
                "+.kuwo.cn",
                "+.msftconnecttest.com",
                "+.msftncsi.com"
            ]));
            changes.push("🚫 配置Fake-IP排除列表".to_string());
        }
        
        Ok(())
    }
    
    /// 标准化性能配置
    fn standardize_performance(config: &mut JsonValue, changes: &mut Vec<String>) -> Result<()> {
        // 布尔配置
        let bool_configs = [
            ("unified-delay", true, "统一延迟测试"),
            ("tcp-concurrent", true, "TCP并发连接"),
        ];
        
        for (key, expected_bool, description) in bool_configs {
            if config.get(key).and_then(|v| v.as_bool()) != Some(expected_bool) {
                config[key] = JsonValue::Bool(expected_bool);
                changes.push(format!("⚡ 启用{}", description));
            }
        }
        
        // 字符串配置
        let string_configs = [
            ("find-process-mode", "strict", "严格进程匹配"),
            ("global-client-fingerprint", "chrome", "Chrome客户端指纹"),
        ];
        
        for (key, expected_str, description) in string_configs {
            if config.get(key).and_then(|v| v.as_str()) != Some(expected_str) {
                config[key] = JsonValue::String(expected_str.to_string());
                changes.push(format!("🎯 设置{}", description));
            }
        }
        
        // Keep-Alive间隔
        if config.get("keep-alive-interval").and_then(|v| v.as_u64()).unwrap_or(0) < 30 {
            config["keep-alive-interval"] = JsonValue::Number(30.into());
            changes.push("🔄 优化Keep-Alive间隔".to_string());
        }
        
        Ok(())
    }
    
    /// 标准化路由规则
    fn standardize_rules(config: &mut JsonValue, changes: &mut Vec<String>) -> Result<()> {
        let standard_rules = vec![
            "DOMAIN-SUFFIX,local,DIRECT",
            "DOMAIN-SUFFIX,localhost,DIRECT", 
            "IP-CIDR,127.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,172.16.0.0/12,DIRECT,no-resolve",
            "IP-CIDR,192.168.0.0/16,DIRECT,no-resolve",
            "IP-CIDR,10.0.0.0/8,DIRECT,no-resolve",
            "IP-CIDR,17.0.0.0/8,DIRECT,no-resolve", 
            "IP-CIDR,100.64.0.0/10,DIRECT,no-resolve",
            "GEOIP,LAN,DIRECT,no-resolve",
            "GEOSITE,category-ads-all,ADBLOCK",
            "GEOSITE,private,DIRECT",
            "GEOSITE,cn,DIRECT",
            "GEOSITE,apple-cn,DIRECT",
            "GEOSITE,microsoft@cn,DIRECT",
            "GEOSITE,steam@cn,DIRECT",
            "GEOSITE,category-games@cn,DIRECT",
            "GEOSITE,geolocation-!cn,PROXY",
            "GEOIP,CN,DIRECT,no-resolve",
            "MATCH,PROXY"
        ];
        
        if !config.as_object().unwrap_or(&serde_json::Map::new()).contains_key("rules") {
            config["rules"] = JsonValue::Array(
                standard_rules.into_iter()
                    .map(|r| JsonValue::String(r.to_string()))
                    .collect()
            );
            changes.push("📋 设置标准化路由规则".to_string());
        } else {
            // 验证和修复现有规则
            let empty_vec = vec![];
            let current_rules = config["rules"].as_array().unwrap_or(&empty_vec);
            let rules_str: Vec<String> = current_rules.iter()
                .filter_map(|r| r.as_str().map(String::from))
                .collect();
            
            // 检查关键规则是否缺失
            let mut missing_rules = Vec::new();
            for rule in &standard_rules {
                if !rules_str.iter().any(|r| r == rule) {
                    missing_rules.push(rule.to_string());
                }
            }
            
            if !missing_rules.is_empty() {
                // 合并规则 - 保留用户规则，补充标准规则
                let mut new_rules = rules_str;
                new_rules.extend(missing_rules.clone());
                
                config["rules"] = JsonValue::Array(
                    new_rules.into_iter()
                        .map(JsonValue::String)
                        .collect()
                );
                
                changes.push(format!("🔧 补充缺失路由规则: {}", missing_rules.len()));
            }
        }
        
        Ok(())
    }
    
    /// 标准化代理组
    fn standardize_proxy_groups(config: &mut JsonValue, changes: &mut Vec<String>) -> Result<()> {
        if !config.as_object().unwrap_or(&serde_json::Map::new()).contains_key("proxy-groups") {
            config["proxy-groups"] = JsonValue::Array(vec![]);
        }
        
        let proxy_groups = config["proxy-groups"].as_array().unwrap();
        
        // 检查是否有必要的代理组
        let required_groups = ["PROXY", "ADBLOCK"];
        let existing_groups: Vec<String> = proxy_groups.iter()
            .filter_map(|g| g.get("name").and_then(|n| n.as_str().map(String::from)))
            .collect();
        
        let mut missing_groups = Vec::new();
        for group in required_groups {
            if !existing_groups.contains(&group.to_string()) {
                missing_groups.push(group);
            }
        }
        
        if !missing_groups.is_empty() {
            changes.push(format!("⚠️ 缺少必要代理组: {:?}。请在订阅或手动配置中添加", missing_groups));
        }
        
        Ok(())
    }
    
    /// 标准化TUN配置
    fn standardize_tun(config: &mut JsonValue, changes: &mut Vec<String>) -> Result<()> {
        if !config.as_object().unwrap_or(&serde_json::Map::new()).contains_key("tun") {
            config["tun"] = serde_json::json!({
                "enable": false,
                "stack": "system",
                "auto-route": true,
                "auto-detect-interface": true,
                "dns-hijack": ["any:53"],
                "mtu": 1500
            });
            changes.push("🔌 配置TUN模式默认设置".to_string());
        }
        
        Ok(())
    }
    
    /// 验证配置完整性
    pub fn validate_config(config: &JsonValue) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        
        // 检查DNS配置
        if let Some(dns) = config.get("dns") {
            if !dns.get("enable").and_then(|v| v.as_bool()).unwrap_or(false) {
                issues.push("❌ DNS服务未启用".to_string());
            }
            
            if dns.get("ipv6").and_then(|v| v.as_bool()).unwrap_or(false) {
                issues.push("⚠️ IPv6已启用，可能影响解析速度".to_string());
            }
            
            if !dns.as_object().unwrap_or(&serde_json::Map::new()).contains_key("nameserver-policy") {
                issues.push("❌ 缺少DNS智能分流配置".to_string());
            }
        } else {
            issues.push("❌ 缺少DNS配置".to_string());
        }
        
        // 检查代理组
        if let Some(groups) = config.get("proxy-groups").and_then(|g| g.as_array()) {
            let group_names: Vec<String> = groups.iter()
                .filter_map(|g| g.get("name").and_then(|n| n.as_str().map(String::from)))
                .collect();
            
            if !group_names.contains(&"PROXY".to_string()) {
                issues.push("⚠️ 缺少PROXY代理组".to_string());
            }
            
            if !group_names.contains(&"ADBLOCK".to_string()) {
                issues.push("⚠️ 缺少ADBLOCK代理组（广告屏蔽）".to_string());
            }
        }
        
        // 检查路由规则
        if let Some(rules) = config.get("rules").and_then(|r| r.as_array()) {
            let rule_strs: Vec<String> = rules.iter()
                .filter_map(|r| r.as_str().map(String::from))
                .collect();
            
            let required_patterns = ["GEOSITE,geolocation-!cn,PROXY", "GEOSITE,category-ads-all"];
            for pattern in required_patterns {
                if !rule_strs.iter().any(|r| r.contains(pattern)) {
                    issues.push(format!("⚠️ 缺少关键路由规则: {}", pattern));
                }
            }
        }
        
        Ok(issues)
    }
    
    /// 生成配置报告
    pub fn generate_report(config: &JsonValue) -> Result<String> {
        let mut report = String::new();
        
        report.push_str("# 📊 配置分析报告\n\n");
        
        // 基本信息
        if let Some(version) = config.get("config_version").and_then(|v| v.as_str()) {
            report.push_str(&format!("**配置版本**: {}\n", version));
        }
        
        // DNS分析
        report.push_str("\n## 🧠 DNS配置分析\n");
        if let Some(dns) = config.get("dns") {
            let enabled = dns.get("enable").and_then(|v| v.as_bool()).unwrap_or(false);
            let ipv6 = dns.get("ipv6").and_then(|v| v.as_bool()).unwrap_or(false);
            let h3 = dns.get("prefer-h3").and_then(|v| v.as_bool()).unwrap_or(false);
            
            report.push_str(&format!("- DNS状态: {}\n", if enabled { "✅ 已启用" } else { "❌ 未启用" }));
            report.push_str(&format!("- IPv6: {}\n", if ipv6 { "⚠️ 已启用" } else { "✅ 已禁用" }));
            report.push_str(&format!("- HTTP/3: {}\n", if h3 { "✅ 已启用" } else { "❌ 未启用" }));
            
            if dns.as_object().unwrap_or(&serde_json::Map::new()).contains_key("nameserver-policy") {
                report.push_str("- 智能分流: ✅ 已配置\n");
            } else {
                report.push_str("- 智能分流: ❌ 未配置\n");
            }
        }
        
        // 代理组分析
        report.push_str("\n## 🎛️ 代理组分析\n");
        if let Some(groups) = config.get("proxy-groups").and_then(|g| g.as_array()) {
            report.push_str(&format!("- 代理组数量: {}\n", groups.len()));
            
            let group_names: Vec<String> = groups.iter()
                .filter_map(|g| g.get("name").and_then(|n| n.as_str().map(String::from)))
                .collect();
            
            for name in &group_names {
                report.push_str(&format!("  - {}\n", name));
            }
        }
        
        // 路由规则分析
        report.push_str("\n## 📋 路由规则分析\n");
        if let Some(rules) = config.get("rules").and_then(|r| r.as_array()) {
            report.push_str(&format!("- 规则数量: {}\n", rules.len()));
            
            let rule_types: HashMap<&str, usize> = rules.iter()
                .filter_map(|r| r.as_str())
                .fold(HashMap::new(), |mut acc, rule| {
                    if rule.starts_with("GEOSITE") {
                        *acc.entry("GEOSITE").or_insert(0) += 1;
                    } else if rule.starts_with("GEOIP") {
                        *acc.entry("GEOIP").or_insert(0) += 1;
                    } else if rule.starts_with("DOMAIN") {
                        *acc.entry("DOMAIN").or_insert(0) += 1;
                    } else if rule.starts_with("IP-CIDR") {
                        *acc.entry("IP-CIDR").or_insert(0) += 1;
                    }
                    acc
                });
            
            for (rule_type, count) in rule_types {
                report.push_str(&format!("  - {}: {} 条\n", rule_type, count));
            }
        }
        
        Ok(report)
    }
}
