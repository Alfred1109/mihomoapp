use tauri::AppHandle;

use crate::{base_config, config, events, subscription};

fn emit_config_change_event(app: &AppHandle) {
    events::emit_config_change(
        app,
        events::ConfigChangeEvent {
            config_path: config::get_config_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            timestamp: events::get_current_timestamp(),
        },
    );
}

async fn emit_subscription_update_event(app: &AppHandle, subscription_id: &str, status: &str) {
    let subscription = subscription::get_subscriptions()
        .await
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.id == subscription_id));

    let subscription_name = subscription
        .as_ref()
        .map(|item| item.name.clone())
        .unwrap_or_default();
    let proxy_count = subscription
        .as_ref()
        .map(|item| item.proxy_count)
        .unwrap_or_default();

    events::emit_subscription_update(
        app,
        events::SubscriptionUpdateEvent {
            subscription_id: subscription_id.to_string(),
            subscription_name,
            status: status.to_string(),
            proxy_count,
            timestamp: events::get_current_timestamp(),
        },
    );
}

fn emit_generic_subscription_event(
    app: &AppHandle,
    subscription_id: &str,
    subscription_name: &str,
    status: &str,
) {
    events::emit_subscription_update(
        app,
        events::SubscriptionUpdateEvent {
            subscription_id: subscription_id.to_string(),
            subscription_name: subscription_name.to_string(),
            status: status.to_string(),
            proxy_count: 0,
            timestamp: events::get_current_timestamp(),
        },
    );
}

#[tauri::command]
pub async fn get_mihomo_config() -> Result<serde_json::Value, String> {
    config::load_config()
        .await
        .map_err(|e| format!("Failed to load config: {}", e))
}

#[tauri::command]
pub async fn save_mihomo_config(
    config: serde_json::Value,
    app: AppHandle,
) -> Result<String, String> {
    match crate::config::save_config(config).await {
        Ok(_) => {
            emit_config_change_event(&app);
            Ok("Configuration saved successfully".to_string())
        }
        Err(e) => Err(format!("Failed to save config: {}", e)),
    }
}

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    user_agent: Option<String>,
    use_proxy: bool,
    app: AppHandle,
) -> Result<String, String> {
    subscription::add_subscription(name, url, user_agent, use_proxy)
        .await
        .map_err(|e| format!("Failed to add subscription: {}", e))
        .map(|id| {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                emit_subscription_update_event(&app_handle, &id, "Added").await;
            });
            "Subscription added successfully".to_string()
        })
}

#[tauri::command]
pub async fn get_subscriptions() -> Result<Vec<subscription::Subscription>, String> {
    subscription::get_subscriptions()
        .await
        .map_err(|e| format!("Failed to get subscriptions: {}", e))
}

#[tauri::command]
pub async fn update_subscription(id: String, app: AppHandle) -> Result<String, String> {
    subscription::update_subscription(&id)
        .await
        .map_err(|e| format!("Failed to update subscription: {}", e))
        .map(|_| {
            let app_handle = app.clone();
            let subscription_id = id.clone();
            tauri::async_runtime::spawn(async move {
                emit_subscription_update_event(&app_handle, &subscription_id, "Updated").await;
                emit_config_change_event(&app_handle);
            });
            "Subscription updated successfully".to_string()
        })
}

#[tauri::command]
pub async fn delete_subscription(id: String, app: AppHandle) -> Result<String, String> {
    subscription::delete_subscription(&id)
        .await
        .map_err(|e| format!("Failed to delete subscription: {}", e))
        .map(|_| {
            emit_generic_subscription_event(&app, &id, "", "Deleted");
            "Subscription deleted successfully".to_string()
        })
}

#[tauri::command]
pub async fn generate_config_from_subscriptions(
    subscription_ids: Vec<String>,
    app: AppHandle,
) -> Result<String, String> {
    subscription::generate_config_from_subscriptions(subscription_ids)
        .await
        .map_err(|e| format!("Failed to generate config: {}", e))
        .map(|_| {
            emit_config_change_event(&app);
            "Configuration generated successfully".to_string()
        })
}

#[tauri::command]
pub async fn enable_tun_mode(enable: bool, app: AppHandle) -> Result<String, String> {
    config::set_tun_mode(enable)
        .await
        .map_err(|e| format!("Failed to set TUN mode: {}", e))
        .map(|_| {
            emit_config_change_event(&app);
            if enable {
                "TUN mode enabled"
            } else {
                "TUN mode disabled"
            }
            .to_string()
        })
}

#[tauri::command]
pub async fn reset_config_to_default(app: AppHandle) -> Result<String, String> {
    config::reset_to_default_config()
        .await
        .map_err(|e| format!("恢复默认配置失败: {}", e))
        .map(|message| {
            emit_config_change_event(&app);
            message
        })
}

#[tauri::command]
pub async fn export_subscriptions() -> Result<String, String> {
    subscription::export_subscriptions()
        .await
        .map_err(|e| format!("导出订阅链接失败: {}", e))
}

#[tauri::command]
pub async fn import_subscriptions(json_content: String, app: AppHandle) -> Result<u32, String> {
    subscription::import_subscriptions(&json_content)
        .await
        .map_err(|e| format!("导入订阅链接失败: {}", e))
        .map(|count| {
            emit_generic_subscription_event(&app, "__all__", "subscriptions", "Imported");
            count
        })
}

#[tauri::command]
pub async fn list_subscription_backups() -> Result<Vec<String>, String> {
    subscription::list_subscription_backups()
        .await
        .map_err(|e| format!("获取订阅备份列表失败: {}", e))
}

#[tauri::command]
pub async fn restore_subscriptions_from_backup(
    backup_filename: String,
    app: AppHandle,
) -> Result<u32, String> {
    subscription::restore_subscriptions_from_backup(&backup_filename)
        .await
        .map_err(|e| format!("恢复订阅链接失败: {}", e))
        .map(|count| {
            emit_generic_subscription_event(&app, "__all__", "subscriptions", "Restored");
            count
        })
}

#[tauri::command]
pub async fn export_base_config() -> Result<String, String> {
    base_config::export_base_config()
        .await
        .map_err(|e| format!("导出基础配置失败: {}", e))
}

#[tauri::command]
pub async fn import_base_config(yaml_content: String, app: AppHandle) -> Result<String, String> {
    base_config::import_base_config(&yaml_content)
        .await
        .map_err(|e| format!("导入基础配置失败: {}", e))?;
    emit_config_change_event(&app);
    Ok("基础配置导入成功".to_string())
}

#[tauri::command]
pub async fn get_base_config() -> Result<serde_json::Value, String> {
    base_config::load_base_config()
        .await
        .map_err(|e| format!("加载基础配置失败: {}", e))
}

#[tauri::command]
pub async fn save_base_config(config: serde_json::Value, app: AppHandle) -> Result<String, String> {
    base_config::save_base_config(&config)
        .await
        .map_err(|e| format!("保存基础配置失败: {}", e))?;
    emit_config_change_event(&app);
    Ok("基础配置保存成功".to_string())
}

#[tauri::command]
pub async fn regenerate_runtime_config(app: AppHandle) -> Result<String, String> {
    let subscriptions = subscription::get_subscriptions()
        .await
        .map_err(|e| format!("获取订阅失败: {}", e))?;

    let active_ids: Vec<String> = subscriptions
        .iter()
        .filter(|s| {
            matches!(s.status, subscription::SubscriptionStatus::Active) && s.proxy_count > 0
        })
        .map(|s| s.id.clone())
        .collect();

    if active_ids.is_empty() {
        return Err("没有活动的订阅，无法重新生成配置".to_string());
    }

    subscription::generate_config_from_subscriptions(active_ids)
        .await
        .map_err(|e| format!("重新生成配置失败: {}", e))?;
    emit_config_change_event(&app);

    Ok("运行时配置已重新生成".to_string())
}
