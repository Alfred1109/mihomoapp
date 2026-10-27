use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use yaml_rust::YamlLoader;

struct FileGuard {
    file: File,
}

impl FileGuard {
    fn new(file: File) -> Self {
        Self { file }
    }
}

impl Drop for FileGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

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

        let file = File::open(&self.config_path).context("Failed to open config file")?;
        FileExt::lock_shared(&file).context("Failed to acquire shared lock")?;
        let _file_guard = FileGuard::new(file);

        let content =
            std::fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        let yaml_docs = YamlLoader::load_from_str(&content).context("Failed to parse YAML")?;

        if yaml_docs.is_empty() {
            return Err(anyhow::anyhow!("YAML file is empty"));
        }

        let yaml_value = &yaml_docs[0];
        let json_value =
            crate::config::yaml_to_json(yaml_value).context("Failed to convert YAML to JSON")?;

        info!("Config read successfully: {:?}", self.config_path);

        Ok(json_value)
    }

    pub async fn write_config(&self, config: serde_json::Value) -> Result<()> {
        self.write_config_with_options(config, true).await
    }

    pub async fn write_validated_config(&self, mut config: serde_json::Value) -> Result<()> {
        mihomoapp::config_compat::normalize_runtime_config(&mut config)?;
        // Reject invalid candidates before touching the current file or its backups.
        let validation = crate::validator::validate_config(&config)
            .await
            .context("Failed to validate configuration")?;
        if !validation.valid {
            return Err(anyhow::anyhow!(
                "配置验证失败: {}",
                validation.errors.join(", ")
            ));
        }
        for warning in validation.warnings {
            warn!("配置警告: {}", warning);
        }

        self.write_config(config).await
    }

    pub async fn write_config_with_options(
        &self,
        config: serde_json::Value,
        create_backup: bool,
    ) -> Result<()> {
        let _guard = self.lock.write().await;
        self.write_config_locked(config, create_backup).await
    }

    pub async fn write_config_if_unchanged(
        &self,
        config: serde_json::Value,
        expected: &[u8],
    ) -> Result<()> {
        let _guard = self.lock.write().await;
        if std::fs::read(&self.config_path).context("读取迁移前配置失败")? != expected {
            return Err(anyhow::anyhow!(
                "配置在兼容校验期间已变化，未覆盖当前文件，请重试"
            ));
        }
        self.write_config_locked(config, true).await
    }

    async fn write_config_locked(
        &self,
        mut config: serde_json::Value,
        create_backup: bool,
    ) -> Result<()> {
        mihomoapp::config_compat::normalize_runtime_config(&mut config)?;
        let yaml_value: serde_yaml::Value =
            serde_json::from_value(config).context("Failed to convert from JSON")?;
        let yaml_content =
            serde_yaml::to_string(&yaml_value).context("Failed to serialize YAML")?;

        let parent = self
            .config_path
            .parent()
            .context("Failed to get parent dir")?;
        std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        let temp_path = parent.join(format!(".config-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut temp_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)
                .context("Failed to create temp file")?;
            temp_file
                .write_all(yaml_content.as_bytes())
                .context("Failed to write temp file")?;
            temp_file.sync_all().context("Failed to sync temp file")?;
            drop(temp_file);
            Ok(())
        })();
        if let Err(error) = result {
            let _ = std::fs::remove_file(&temp_path);
            return Err(error);
        }

        // A failed backup must leave the live file untouched as well.
        if create_backup {
            if let Err(error) = self.create_backup().await {
                let _ = std::fs::remove_file(&temp_path);
                return Err(error);
            }
        }
        if let Err(error) = std::fs::rename(&temp_path, &self.config_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(error).context("Failed to replace config file");
        }

        info!("Config saved successfully: {:?}", self.config_path);

        Ok(())
    }

    async fn create_backup(&self) -> Result<()> {
        if !self.config_path.exists() {
            return Ok(());
        }

        let backup_dir = self
            .config_path
            .parent()
            .context("Failed to get parent dir")?
            .join("backups");

        std::fs::create_dir_all(&backup_dir).context("Failed to create backup dir")?;

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!(
            "config_{}_{}.yaml",
            timestamp,
            uuid::Uuid::new_v4()
        ));

        std::fs::copy(&self.config_path, &backup_path).context("Failed to create backup")?;

        info!("Backup created: {:?}", backup_path);

        self.cleanup_old_backups(&backup_dir).await?;

        Ok(())
    }

    async fn cleanup_old_backups(&self, backup_dir: &PathBuf) -> Result<()> {
        let mut backups: Vec<_> = std::fs::read_dir(backup_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "yaml")
                    .unwrap_or(false)
            })
            .collect();

        backups.sort_by_key(|entry| {
            entry
                .metadata()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("mihomo-config-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn candidate() -> serde_json::Value {
        json!({
            "port": 7890,
            "socks-port": 7891,
            "external-controller": "127.0.0.1:9090",
            "proxies": [{"name": "node", "type": "ss", "server": "example.com"}]
        })
    }

    #[tokio::test]
    async fn invalid_candidate_preserves_live_config_and_backups() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let manager = ConfigManager::new(path.clone());
        manager.write_config(candidate()).await.unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut invalid = candidate();
        let duplicate = invalid["proxies"][0].clone();
        invalid["proxies"].as_array_mut().unwrap().push(duplicate);
        assert!(manager.write_validated_config(invalid).await.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!dir.0.join("backups").exists());
    }

    #[tokio::test]
    async fn failed_backup_preserves_live_config_and_removes_candidate() {
        let dir = TestDirectory::new();
        let path = dir.0.join("config.yaml");
        let manager = ConfigManager::new(path.clone());
        manager.write_config(candidate()).await.unwrap();
        let original = std::fs::read(&path).unwrap();
        // A file at the backup directory path makes backup creation fail.
        std::fs::write(dir.0.join("backups"), "blocked").unwrap();
        let mut replacement = candidate();
        replacement["port"] = json!(7892);
        assert!(manager.write_config(replacement).await.is_err());
        assert_eq!(std::fs::read(path).unwrap(), original);
        assert!(std::fs::read_dir(&dir.0).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
    }

    #[tokio::test]
    async fn rapid_updates_keep_distinct_recoverable_backups() {
        let dir = TestDirectory::new();
        let manager = ConfigManager::new(dir.0.join("config.yaml"));
        let mut config = candidate();
        for port in [7890, 7892, 7893] {
            config["port"] = json!(port);
            manager.write_config(config.clone()).await.unwrap();
        }
        let backups: Vec<_> = std::fs::read_dir(dir.0.join("backups"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(backups.len(), 2);
        let mut ports: Vec<_> = backups
            .iter()
            .map(|path| {
                let value: serde_yaml::Value =
                    serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
                value["port"].as_u64().unwrap()
            })
            .collect();
        ports.sort_unstable();
        assert_eq!(ports, vec![7890, 7892]);
        assert_eq!(manager.read_config().await.unwrap()["port"], json!(7893));
        assert!(std::fs::read_dir(&dir.0).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
    }
}
