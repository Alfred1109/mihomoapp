use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub windows: Option<PlatformSettings>,
    pub linux: Option<PlatformSettings>,
    pub macos: Option<PlatformSettings>,
    pub common: CommonSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSettings {
    pub mihomo_binary: String,
    pub service_name: String,
    #[serde(default)]
    pub service_wrapper: Option<String>,
    
    pub config_dir: String,
    pub backup_dir: String,
    pub log_dir: String,
    
    #[serde(default)]
    pub system_paths: Vec<String>,
    
    pub kill_command: String,
    pub kill_args: Vec<String>,
    
    pub service_install_command: String,
    pub service_start_command: String,
    pub service_stop_command: String,
    pub service_query_command: String,
    
    #[serde(default)]
    pub service_unit_dir: Option<String>,
    #[serde(default)]
    pub service_plist_dir: Option<String>,
    
    #[serde(default)]
    pub autostart_registry_key: Option<String>,
    #[serde(default)]
    pub autostart_value_name: Option<String>,
    #[serde(default)]
    pub autostart_dir: Option<String>,
    #[serde(default)]
    pub autostart_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonSettings {
    pub config_filename: String,
    pub backup_prefix: String,
    pub max_backups: usize,
    pub api_host: String,
    pub api_port: u16,
}

lazy_static! {
    static ref PLATFORM_CONFIG: Mutex<Option<PlatformConfig>> = Mutex::new(None);
}

impl PlatformConfig {
    /// 加载平台配置
    pub fn load() -> Result<Self> {
        // 尝试从嵌入的配置加载
        let config_str = include_str!("../platform_config.toml");
        let config: PlatformConfig = toml::from_str(config_str)
            .context("Failed to parse platform config")?;
        
        Ok(config)
    }
    
    /// 获取当前平台的配置
    pub fn current_platform() -> Result<PlatformSettings> {
        let config = Self::get_or_load()?;
        
        #[cfg(target_os = "windows")]
        {
            config.windows
                .ok_or_else(|| anyhow::anyhow!("Windows platform config not found"))
        }
        
        #[cfg(target_os = "linux")]
        {
            config.linux
                .ok_or_else(|| anyhow::anyhow!("Linux platform config not found"))
        }
        
        #[cfg(target_os = "macos")]
        {
            config.macos
                .ok_or_else(|| anyhow::anyhow!("macOS platform config not found"))
        }
    }
    
    /// 获取通用配置
    pub fn common() -> Result<CommonSettings> {
        let config = Self::get_or_load()?;
        Ok(config.common)
    }
    
    /// 获取或加载配置（单例模式）
    fn get_or_load() -> Result<PlatformConfig> {
        let mut config_lock = PLATFORM_CONFIG.lock().unwrap();
        
        if config_lock.is_none() {
            *config_lock = Some(Self::load()?);
        }
        
        Ok(config_lock.as_ref().unwrap().clone())
    }
}

/// 路径解析器 - 支持环境变量替换
pub struct PathResolver;

impl PathResolver {
    /// 解析路径中的环境变量
    /// 支持: ${VAR}, $VAR, %VAR% (Windows)
    pub fn resolve(path: &str) -> Result<PathBuf> {
        let mut resolved = path.to_string();
        
        // 处理 ${VAR} 格式
        while let Some(start) = resolved.find("${") {
            if let Some(end) = resolved[start..].find('}') {
                let var_name = &resolved[start + 2..start + end];
                let value = Self::get_env_var(var_name)?;
                resolved = format!("{}{}{}", 
                    &resolved[..start], 
                    value, 
                    &resolved[start + end + 1..]
                );
            } else {
                break;
            }
        }
        
        // 处理 Windows %VAR% 格式
        #[cfg(target_os = "windows")]
        {
            while let Some(start) = resolved.find('%') {
                if let Some(end) = resolved[start + 1..].find('%') {
                    let var_name = &resolved[start + 1..start + 1 + end];
                    if let Ok(value) = Self::get_env_var(var_name) {
                        resolved = format!("{}{}{}", 
                            &resolved[..start], 
                            value, 
                            &resolved[start + end + 2..]
                        );
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        
        Ok(PathBuf::from(resolved))
    }
    
    /// 获取环境变量，支持特殊变量
    fn get_env_var(var_name: &str) -> Result<String> {
        match var_name {
            "HOME" => {
                // 优先使用 HOME 环境变量
                if let Ok(home) = std::env::var("HOME") {
                    return Ok(home);
                }
                
                // 如果是 sudo 运行，尝试获取真实用户的 HOME
                #[cfg(not(target_os = "windows"))]
                {
                    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
                        if sudo_user != "root" {
                            return Ok(format!("/home/{}", sudo_user));
                        }
                    }
                }
                
                // 使用 dirs 库获取
                dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .ok_or_else(|| anyhow::anyhow!("Failed to get HOME directory"))
            }
            
            "APPDATA" => {
                #[cfg(target_os = "windows")]
                {
                    dirs::config_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .ok_or_else(|| anyhow::anyhow!("Failed to get APPDATA directory"))
                }
                
                #[cfg(not(target_os = "windows"))]
                {
                    Err(anyhow::anyhow!("APPDATA is only available on Windows"))
                }
            }
            
            "USER" => {
                std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .context("Failed to get USER")
            }
            
            "SUDO_USER" => {
                std::env::var("SUDO_USER")
                    .context("SUDO_USER not set")
            }
            
            _ => {
                // 尝试从环境变量获取
                std::env::var(var_name)
                    .with_context(|| format!("Environment variable {} not found", var_name))
            }
        }
    }
    
    /// 确保目录存在
    pub fn ensure_dir(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        }
        Ok(())
    }
}

/// 平台路径管理器
pub struct PlatformPaths;

impl PlatformPaths {
    /// 获取配置目录
    pub fn config_dir() -> Result<PathBuf> {
        let platform = PlatformConfig::current_platform()?;
        let path = PathResolver::resolve(&platform.config_dir)?;
        PathResolver::ensure_dir(&path)?;
        Ok(path)
    }
    
    /// 获取配置文件路径
    pub fn config_file() -> Result<PathBuf> {
        let config_dir = Self::config_dir()?;
        let common = PlatformConfig::common()?;
        Ok(config_dir.join(&common.config_filename))
    }
    
    /// 获取备份目录
    pub fn backup_dir() -> Result<PathBuf> {
        let platform = PlatformConfig::current_platform()?;
        let path = PathResolver::resolve(&platform.backup_dir)?;
        PathResolver::ensure_dir(&path)?;
        Ok(path)
    }
    
    /// 获取日志目录
    #[allow(dead_code)]
    pub fn log_dir() -> Result<PathBuf> {
        let platform = PlatformConfig::current_platform()?;
        let path = PathResolver::resolve(&platform.log_dir)?;
        PathResolver::ensure_dir(&path)?;
        Ok(path)
    }
    
    /// 获取系统路径列表
    #[allow(dead_code)]
    pub fn system_paths() -> Result<Vec<PathBuf>> {
        let platform = PlatformConfig::current_platform()?;
        platform.system_paths
            .iter()
            .map(|p| PathResolver::resolve(p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_config() {
        let config = PlatformConfig::load();
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert!(config.windows.is_some());
        assert!(config.linux.is_some());
        assert_eq!(config.common.api_port, 9090);
    }
    
    #[test]
    fn test_path_resolver() {
        std::env::set_var("TEST_VAR", "/test/path");
        
        let resolved = PathResolver::resolve("${TEST_VAR}/config");
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap().to_str().unwrap(), "/test/path/config");
    }
    
    #[test]
    fn test_current_platform() {
        let platform = PlatformConfig::current_platform();
        assert!(platform.is_ok());
        
        let platform = platform.unwrap();
        assert!(!platform.mihomo_binary.is_empty());
    }
}
