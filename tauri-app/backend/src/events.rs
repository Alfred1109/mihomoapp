use serde::{Deserialize, Serialize};
use tauri::Manager;
use tracing::warn;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MihomoStatusEvent {
    pub running: bool,
    pub process_id: Option<u32>,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConfigChangeEvent {
    pub config_path: String,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProxyChangeEvent {
    pub group_name: String,
    pub proxy_name: String,
    pub timestamp: u64,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SubscriptionUpdateEvent {
    pub subscription_id: String,
    pub subscription_name: String,
    pub status: String,
    pub proxy_count: u32,
    pub timestamp: u64,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ErrorEvent {
    pub code: String,
    pub message: String,
    pub source: String,
    pub timestamp: u64,
}

#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NotificationEvent {
    pub message: String,
    pub severity: String,
    pub timestamp: u64,
}

pub fn emit_mihomo_status(app: &tauri::AppHandle, status: MihomoStatusEvent) {
    if let Err(e) = app.emit_all("mihomo-status", status) {
        warn!("Failed to emit mihomo-status event: {}", e);
    }
}

pub fn emit_config_change(app: &tauri::AppHandle, event: ConfigChangeEvent) {
    if let Err(e) = app.emit_all("config-change", event) {
        warn!("Failed to emit config-change event: {}", e);
    }
}

pub fn emit_proxy_change(app: &tauri::AppHandle, event: ProxyChangeEvent) {
    if let Err(e) = app.emit_all("proxy-change", event) {
        warn!("Failed to emit proxy-change event: {}", e);
    }
}

#[allow(dead_code)]
pub fn emit_subscription_update(app: &tauri::AppHandle, event: SubscriptionUpdateEvent) {
    if let Err(e) = app.emit_all("subscription-update", event) {
        warn!("Failed to emit subscription-update event: {}", e);
    }
}

#[allow(dead_code)]
pub fn emit_error(app: &tauri::AppHandle, event: ErrorEvent) {
    if let Err(e) = app.emit_all("app-error", event) {
        warn!("Failed to emit app-error event: {}", e);
    }
}

#[allow(dead_code)]
pub fn emit_notification(app: &tauri::AppHandle, event: NotificationEvent) {
    if let Err(e) = app.emit_all("app-notification", event) {
        warn!("Failed to emit app-notification event: {}", e);
    }
}

pub fn get_current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[allow(dead_code)]
pub fn create_subscription_update_event(
    id: &str,
    name: &str,
    status: &str,
    proxy_count: u32,
) -> SubscriptionUpdateEvent {
    SubscriptionUpdateEvent {
        subscription_id: id.to_string(),
        subscription_name: name.to_string(),
        status: status.to_string(),
        proxy_count,
        timestamp: get_current_timestamp(),
    }
}

#[allow(dead_code)]
pub fn create_error_event(code: &str, message: &str, source: &str) -> ErrorEvent {
    ErrorEvent {
        code: code.to_string(),
        message: message.to_string(),
        source: source.to_string(),
        timestamp: get_current_timestamp(),
    }
}

#[allow(dead_code)]
pub fn create_notification_event(message: &str, severity: &str) -> NotificationEvent {
    NotificationEvent {
        message: message.to_string(),
        severity: severity.to_string(),
        timestamp: get_current_timestamp(),
    }
}
