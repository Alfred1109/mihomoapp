use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;
use anyhow::{Result, Context};
use fs2::FileExt;
use std::fs::File;
use tracing::{info, warn};
use yaml_rust::YamlLoader;

pub struct ConfigManager {
    config_path: PathBuf,
    lock: Arc<RwLock<()>>,
}

impl ConfigManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            lock: Arc::new(RwLock::new(())),
        }
    }
    
    pub async fn read_config(&self) -> Result<serde_json::Value> {
        let _guard = self.lock.read().await;
        
        if !self.config_path.exists() {
            return Err(anyhow::anyhow!("Config file does not exist"));
        }
        
        let file = File::open(&self.config_path)
            .context("Failed to open config file")?;
        
        file.lock_shared()
            .context("Failed to acquire shared lock")?;
        
        let content = std::fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;
        
        // Use yaml-rust which supports multi-document YAML
        let yaml_docs = YamlLoader::load_from_str(&content)
            .context("Failed to parse YAML")?;
        
        if yaml_docs.is_empty() {
            return Err(anyhow::anyhow!("YAML file is empty"));
        }
        
        // Only use the first document
        let yaml_value = &yaml_docs[0];
        let json_value = crate::config::yaml_to_json(yaml_value)
            .context("Failed to convert YAML to JSON")?;
        
        file.unlock().ok();
        
        info!("Config read successfully: {:?}", self.config_path);
        
        Ok(json_value)
    }
    
    pub async fn write_config(&self, config: serde_json::Value) -> Result<()> {
        let _guard = self.lock.write().await;
        
        self.create_backup().await?;
        
        let yaml_value: serde_yaml::Value = serde_json::from_value(config)
            .context("Failed to convert from JSON")?;
        
        let yaml_content = serde_yaml::to_string(&yaml_value)
            .context("Failed to serialize YAML")?;
        
        let temp_path = self.config_path.with_extension("yaml.tmp");
        
        // 先写入临时文件
        std::fs::write(&temp_path, yaml_content)
            .context("Failed to write temp file")?;
        
        // 然后打开文件进行同步和锁定
        {
            use std::fs::OpenOptions;
            let temp_file = OpenOptions::new()
                .write(true)
                .open(&temp_path)
                .context("Failed to open temp file")?;
            
            temp_file.lock_exclusive()
                .context("Failed to acquire exclusive lock")?;
            
            temp_file.sync_all()
                .context("Failed to sync temp file")?;
            
            temp_file.unlock().ok();
        }
        
        std::fs::rename(&temp_path, &self.config_path)
            .context("Failed to rename temp file")?;
        
        info!("Config saved successfully: {:?}", self.config_path);
        
        Ok(())
    }
    
    async fn create_backup(&self) -> Result<()> {
        if !self.config_path.exists() {
            return Ok(());
        }
        
        let backup_dir = self.config_path.parent()
            .context("Failed to get parent dir")?
            .join("backups");
        
        std::fs::create_dir_all(&backup_dir)
            .context("Failed to create backup dir")?;
        
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("config_{}.yaml", timestamp));
        
        std::fs::copy(&self.config_path, &backup_path)
            .context("Failed to create backup")?;
        
        info!("Backup created: {:?}", backup_path);
        
        self.cleanup_old_backups(&backup_dir).await?;
        
        Ok(())
    }
    
    async fn cleanup_old_backups(&self, backup_dir: &PathBuf) -> Result<()> {
        let mut backups: Vec<_> = std::fs::read_dir(backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "yaml")
                    .unwrap_or(false)
            })
            .collect();
        
        backups.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        
        backups.reverse();
        
        const MAX_BACKUPS: usize = 10;
        
        if backups.len() > MAX_BACKUPS {
            for backup in backups.iter().skip(MAX_BACKUPS) {
                if let Err(e) = std::fs::remove_file(backup.path()) {
                    warn!("Failed to remove old backup {:?}: {}", backup.path(), e);
                } else {
                    info!("Removed old backup: {:?}", backup.path());
                }
            }
        }
        
        Ok(())
    }
}

lazy_static::lazy_static! {
    static ref CONFIG_MANAGER: Arc<RwLock<Option<ConfigManager>>> = Arc::new(RwLock::new(None));
}

pub async fn init_config_manager(config_path: PathBuf) {
    let path_clone = config_path.clone();
    let mut manager = CONFIG_MANAGER.write().await;
    *manager = Some(ConfigManager::new(config_path));
    info!("ConfigManager initialized with path: {:?}", path_clone);
}

pub async fn get_config_manager() -> Result<ConfigManager> {
    let manager_lock = CONFIG_MANAGER.read().await;
    match manager_lock.as_ref() {
        Some(manager) => Ok(ConfigManager {
            config_path: manager.config_path.clone(),
            lock: manager.lock.clone(),
        }),
        None => Err(anyhow::anyhow!("ConfigManager not initialized")),
    }
}
