use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};
use crate::config_validator::ConfigStandardizer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSyncResult {
    pub success: bool,
    pub changes: Vec<String>,
    pub issues: Vec<String>,
    pub backup_created: bool,
    pub config_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigStatus {
    pub is_standardized: bool,
    pub version: String,
    pub issues_count: usize,
    pub last_sync: Option<String>,
    pub platform: String,
}

/// 配置同步管理器
pub struct ConfigSyncManager;

impl ConfigSyncManager {
    /// 标准化当前配置文件
    pub async fn standardize_config() -> Result<ConfigSyncResult> {
        info!("开始标准化配置文件...");
        
        // 1. 加载当前配置
        let mut config = crate::config::load_config().await
            .context("无法加载当前配置")?;
        
        let config_path = crate::config::get_config_path()?;
        
        // 2. 创建备份
        let backup_created = Self::create_backup(&config_path).await.is_ok();
        
        // 3. 标准化配置
        let changes = ConfigStandardizer::standardize_config(&mut config)
            .context("配置标准化失败")?;
        
        // 4. 验证配置
        let issues = ConfigStandardizer::validate_config(&config)
            .context("配置验证失败")?;
        
        // 5. 保存标准化后的配置
        crate::config::save_config(config).await
            .context("保存标准化配置失败")?;
        
        let result = ConfigSyncResult {
            success: true,
            changes,
            issues,
            backup_created,
            config_path: config_path.to_string_lossy().to_string(),
        };
        
        info!("配置标准化完成，变更: {}, 问题: {}", 
              result.changes.len(), result.issues.len());
        
        Ok(result)
    }
    
    /// 检查配置状态
    pub async fn check_config_status() -> Result<ConfigStatus> {
        let config = crate::config::load_config().await
            .context("无法加载配置")?;
        
        // 检查配置版本
        let version = config.get("config_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0")
            .to_string();
        
        // 验证配置
        let issues = ConfigStandardizer::validate_config(&config)
            .unwrap_or_default();
        
        let is_standardized = issues.is_empty() && version == "2.0";
        
        // 获取平台信息
        let platform = Self::get_platform_name();
        
        // 检查最后同步时间（从配置元数据获取）
        let last_sync = config.get("last_standardized")
            .and_then(|v| v.as_str())
            .map(String::from);
        
        Ok(ConfigStatus {
            is_standardized,
            version,
            issues_count: issues.len(),
            last_sync,
            platform,
        })
    }
    
    /// 从示例配置重置
    pub async fn reset_from_template() -> Result<ConfigSyncResult> {
        info!("从示例配置重置...");
        
        let config_path = crate::config::get_config_path()?;
        
        // 1. 创建备份
        let backup_created = Self::create_backup(&config_path).await.is_ok();
        
        // 2. 加载示例配置
        let template_config = Self::load_template_config().await?;
        
        // 3. 保存新配置
        crate::config::save_config(template_config.clone()).await
            .context("保存模板配置失败")?;
        
        // 4. 验证新配置
        let issues = ConfigStandardizer::validate_config(&template_config)
            .unwrap_or_default();
        
        Ok(ConfigSyncResult {
            success: true,
            changes: vec!["🔄 从示例配置重置".to_string()],
            issues,
            backup_created,
            config_path: config_path.to_string_lossy().to_string(),
        })
    }
    
    /// 生成配置报告
    pub async fn generate_report() -> Result<String> {
        let config = crate::config::load_config().await
            .context("无法加载配置")?;
        
        ConfigStandardizer::generate_report(&config)
    }
    
    /// 配置健康检查
    pub async fn health_check() -> Result<serde_json::Value> {
        let config = crate::config::load_config().await?;
        let issues = ConfigStandardizer::validate_config(&config)?;
        let status = Self::check_config_status().await?;
        
        let health_score = if issues.is_empty() { 100 } else {
            std::cmp::max(0, 100 - (issues.len() as i32 * 10))
        };
        
        Ok(serde_json::json!({
            "health_score": health_score,
            "status": status,
            "issues": issues,
            "recommendations": Self::generate_recommendations(&issues),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
    
    /// 跨平台配置迁移
    pub async fn migrate_config(source_config: serde_json::Value) -> Result<ConfigSyncResult> {
        info!("开始跨平台配置迁移...");
        
        let config_path = crate::config::get_config_path()?;
        let backup_created = Self::create_backup(&config_path).await.is_ok();
        
        // 标准化迁移的配置
        let mut migrated_config = source_config;
        let changes = ConfigStandardizer::standardize_config(&mut migrated_config)?;
        
        // 添加平台适配
        Self::adapt_for_platform(&mut migrated_config).await?;
        
        // 验证迁移后的配置
        let issues = ConfigStandardizer::validate_config(&migrated_config)?;
        
        // 保存迁移后的配置
        crate::config::save_config(migrated_config).await?;
        
        Ok(ConfigSyncResult {
            success: true,
            changes,
            issues,
            backup_created,
            config_path: config_path.to_string_lossy().to_string(),
        })
    }
    
    // 辅助方法
    
    async fn create_backup(config_path: &PathBuf) -> Result<()> {
        if !config_path.exists() {
            return Ok(());
        }
        
        let backup_dir = config_path.parent()
            .context("无法获取配置目录")?
            .join("backups");
        
        std::fs::create_dir_all(&backup_dir)
            .context("创建备份目录失败")?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("config_before_sync_{}.yaml", timestamp));
        
        std::fs::copy(config_path, &backup_path)
            .context("创建备份失败")?;
        
        info!("配置备份已创建: {:?}", backup_path);
        Ok(())
    }
    
    async fn load_template_config() -> Result<serde_json::Value> {
        // 优先从项目根目录的 config.example.yaml 加载
        let project_root = std::env::current_exe()
            .context("获取执行文件路径失败")?
            .parent()
            .context("获取执行文件目录失败")?
            .parent()
            .context("获取项目根目录失败")?
            .to_path_buf();
        
        let example_path = project_root.join("config.example.yaml");
        
        if example_path.exists() {
            let content = std::fs::read_to_string(&example_path)
                .context("读取示例配置失败")?;
            
            let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
                .context("解析示例配置失败")?;
            
            serde_json::to_value(yaml_value)
                .context("转换示例配置格式失败")
        } else {
            // fallback到内置默认配置
            warn!("示例配置文件不存在，使用内置默认配置");
            Ok(crate::mihomo::create_default_config())
        }
    }
    
    fn get_platform_name() -> String {
        #[cfg(target_os = "windows")]
        return "Windows".to_string();
        
        #[cfg(target_os = "macos")]
        return "macOS".to_string();
        
        #[cfg(target_os = "linux")]
        return "Linux".to_string();
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return "Unknown".to_string();
    }
    
    async fn adapt_for_platform(config: &mut serde_json::Value) -> Result<()> {
        // 平台特定的配置适配
        
        #[cfg(target_os = "windows")]
        {
            // Windows特定配置
            if let Some(tun) = config.get_mut("tun") {
                if let Some(stack) = tun.get_mut("stack") {
                    *stack = serde_json::Value::String("system".to_string());
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // Linux特定配置
            if let Some(dns) = config.get_mut("dns") {
                if let Some(listen) = dns.get_mut("listen") {
                    // Linux上默认使用1053端口避免权限问题
                    if listen.as_str() == Some("0.0.0.0:53") {
                        *listen = serde_json::Value::String("0.0.0.0:1053".to_string());
                    }
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS特定配置
            if let Some(tun) = config.get_mut("tun") {
                if let Some(stack) = tun.get_mut("stack") {
                    *stack = serde_json::Value::String("system".to_string());
                }
            }
        }
        
        // 添加平台标识和同步时间戳
        config["platform"] = serde_json::Value::String(Self::get_platform_name());
        config["last_standardized"] = serde_json::Value::String(
            chrono::Utc::now().to_rfc3339()
        );
        
        Ok(())
    }
    
    fn generate_recommendations(issues: &[String]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if issues.iter().any(|i| i.contains("DNS")) {
            recommendations.push("🧠 建议运行配置标准化以优化DNS设置".to_string());
        }
        
        if issues.iter().any(|i| i.contains("代理组")) {
            recommendations.push("🎛️ 请添加必要的代理组或使用订阅链接".to_string());
        }
        
        if issues.iter().any(|i| i.contains("路由规则")) {
            recommendations.push("📋 建议更新路由规则以获得最佳分流效果".to_string());
        }
        
        if issues.iter().any(|i| i.contains("IPv6")) {
            recommendations.push("⚡ 建议禁用IPv6以提升DNS解析速度".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("✅ 配置状态良好，无需特殊操作".to_string());
        }
        
        recommendations
    }
}
