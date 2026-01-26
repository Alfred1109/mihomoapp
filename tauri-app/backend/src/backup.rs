use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;

/// 备份配置文件，保留最近5个备份
pub async fn backup_config() -> Result<()> {
    let config_path = get_config_path()?;
    
    if !config_path.exists() {
        return Ok(()); // 配置文件不存在，无需备份
    }
    
    let backup_dir = get_backup_dir()?;
    fs::create_dir_all(&backup_dir)?;
    
    // 生成备份文件名：config.yaml.backup.timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_filename = format!("config.yaml.backup.{}", timestamp);
    let backup_path = backup_dir.join(&backup_filename);
    
    // 复制配置文件到备份目录
    fs::copy(&config_path, &backup_path)
        .context("Failed to backup config file")?;
    
    // 清理旧备份，只保留最近5个
    cleanup_old_backups(&backup_dir, 5)?;
    
    println!("✓ 配置已备份到: {}", backup_path.display());
    Ok(())
}

/// 恢复指定的备份配置
pub async fn restore_config(backup_filename: &str) -> Result<()> {
    let backup_dir = get_backup_dir()?;
    let backup_path = backup_dir.join(backup_filename);
    
    if !backup_path.exists() {
        return Err(anyhow::anyhow!("备份文件不存在: {}", backup_filename));
    }
    
    let config_path = get_config_path()?;
    
    // 在恢复前先备份当前配置
    if config_path.exists() {
        let temp_backup = format!("config.yaml.backup.before_restore_{}", 
            chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let temp_backup_path = backup_dir.join(&temp_backup);
        fs::copy(&config_path, &temp_backup_path)?;
    }
    
    // 恢复备份
    fs::copy(&backup_path, &config_path)
        .context("Failed to restore config from backup")?;
    
    // 清理旧备份，包括 before_restore 临时备份
    cleanup_old_backups(&backup_dir, 5)?;
    
    println!("✓ 配置已从备份恢复: {}", backup_filename);
    Ok(())
}

/// 列出所有可用的备份
pub async fn list_backups() -> Result<Vec<String>> {
    let backup_dir = get_backup_dir()?;
    
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut backups = Vec::new();
    
    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let filename = entry.file_name().to_string_lossy().to_string();
        
        if filename.starts_with("config.yaml.backup.") {
            backups.push(filename);
        }
    }
    
    // 按时间戳降序排序（最新的在前）
    backups.sort_by(|a, b| b.cmp(a));
    
    Ok(backups)
}

/// 清理旧备份，只保留指定数量的最新备份
fn cleanup_old_backups(backup_dir: &PathBuf, keep_count: usize) -> Result<()> {
    let mut backups: Vec<_> = fs::read_dir(backup_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name()
                .to_string_lossy()
                .starts_with("config.yaml.backup.")
        })
        .collect();
    
    // 按修改时间排序（最新的在前）
    backups.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });
    
    // 删除超出保留数量的备份
    for backup in backups.iter().skip(keep_count) {
        if let Err(e) = fs::remove_file(backup.path()) {
            eprintln!("⚠ 删除旧备份失败: {}", e);
        }
    }
    
    Ok(())
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = get_mihomo_config_dir()?;
    Ok(config_dir.join("config.yaml"))
}

fn get_mihomo_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 AppData/Roaming 目录
        Ok(dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
            .join("mihomo"))
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/Unix: 优先使用 SUDO_USER 环境变量获取实际用户的 home 目录
        // 这样即使以 root 运行，也会使用实际用户的配置
        let config_dir = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
            let user_home = PathBuf::from(format!("/home/{}", sudo_user));
            user_home.join(".config").join("mihomo")
        } else if let Ok(user) = std::env::var("USER") {
            if user != "root" {
                let user_home = PathBuf::from(format!("/home/{}", user));
                user_home.join(".config").join("mihomo")
            } else {
                dirs::config_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
                    .join("mihomo")
            }
        } else {
            dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get config directory"))?
                .join("mihomo")
        };
        Ok(config_dir)
    }
}

/// 删除指定的备份文件
pub async fn delete_backup(backup_filename: &str) -> Result<()> {
    let backup_dir = get_backup_dir()?;
    let backup_path = backup_dir.join(backup_filename);
    
    if !backup_path.exists() {
        return Err(anyhow::anyhow!("备份文件不存在: {}", backup_filename));
    }
    
    // 检查是否是备份文件（防止误删其他文件）
    if !backup_filename.starts_with("config.yaml.backup.") {
        return Err(anyhow::anyhow!("无效的备份文件名"));
    }
    
    fs::remove_file(&backup_path)
        .context("Failed to delete backup file")?;
    
    println!("✓ 备份已删除: {}", backup_filename);
    Ok(())
}

/// 重命名备份文件（添加自定义标签）
pub async fn rename_backup(old_filename: &str, new_label: &str) -> Result<String> {
    let backup_dir = get_backup_dir()?;
    let old_path = backup_dir.join(old_filename);
    
    if !old_path.exists() {
        return Err(anyhow::anyhow!("备份文件不存在: {}", old_filename));
    }
    
    // 检查是否是备份文件
    if !old_filename.starts_with("config.yaml.backup.") {
        return Err(anyhow::anyhow!("无效的备份文件名"));
    }
    
    // 提取时间戳部分
    let timestamp = old_filename.strip_prefix("config.yaml.backup.")
        .ok_or_else(|| anyhow::anyhow!("无法解析备份文件名"))?;
    
    // 生成新文件名：config.yaml.backup.timestamp.label
    let safe_label = new_label.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_string();
    
    if safe_label.is_empty() {
        return Err(anyhow::anyhow!("标签不能为空"));
    }
    
    let new_filename = format!("config.yaml.backup.{}.{}", timestamp, safe_label);
    let new_path = backup_dir.join(&new_filename);
    
    // 检查新文件名是否已存在
    if new_path.exists() {
        return Err(anyhow::anyhow!("该标签的备份已存在"));
    }
    
    fs::rename(&old_path, &new_path)
        .context("Failed to rename backup file")?;
    
    println!("✓ 备份已重命名: {} -> {}", old_filename, new_filename);
    Ok(new_filename)
}

fn get_backup_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    
    #[cfg(target_os = "windows")]
    let backup_dir = home.join("AppData").join("Roaming").join("mihomo").join("backups");
    
    #[cfg(not(target_os = "windows"))]
    let backup_dir = home.join(".config").join("mihomo").join("backups");
    
    Ok(backup_dir)
}
