use serde::{Deserialize, Serialize};
use tauri::Manager;

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

pub fn emit_mihomo_status(app: &tauri::AppHandle, status: MihomoStatusEvent) {
    if let Err(e) = app.emit_all("mihomo-status", status) {
        eprintln!("Failed to emit mihomo-status event: {}", e);
    }
}

pub fn emit_config_change(app: &tauri::AppHandle, event: ConfigChangeEvent) {
    if let Err(e) = app.emit_all("config-change", event) {
        eprintln!("Failed to emit config-change event: {}", e);
    }
}

pub fn emit_proxy_change(app: &tauri::AppHandle, event: ProxyChangeEvent) {
    if let Err(e) = app.emit_all("proxy-change", event) {
        eprintln!("Failed to emit proxy-change event: {}", e);
    }
}

pub fn get_current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
