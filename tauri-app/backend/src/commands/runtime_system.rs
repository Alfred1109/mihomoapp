use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{backup, events, mihomo, validator, watchdog};

#[tauri::command]
pub async fn get_mihomo_status() -> Result<bool, String> {
    Ok(mihomo::is_mihomo_running().await)
}

#[tauri::command]
pub async fn get_proxies() -> Result<serde_json::Value, String> {
    mihomo::get_proxies()
        .await
        .map_err(|e| format!("Failed to get proxies: {}", e))
}

#[tauri::command]
pub async fn switch_proxy(
    group_name: String,
    proxy_name: String,
    app: AppHandle,
) -> Result<String, String> {
    match mihomo::switch_proxy(&group_name, &proxy_name).await {
        Ok(_) => {
            events::emit_proxy_change(
                &app,
                events::ProxyChangeEvent {
                    group_name: group_name.clone(),
                    proxy_name: proxy_name.clone(),
                    timestamp: events::get_current_timestamp(),
                },
            );
            Ok("Proxy switched successfully".to_string())
        }
        Err(e) => Err(format!("Failed to switch proxy: {}", e)),
    }
}

#[tauri::command]
pub async fn set_auto_restart(
    enabled: bool,
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    watchdog.set_auto_restart(enabled).await;
    Ok(format!(
        "Auto-restart {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

#[tauri::command]
pub async fn get_auto_restart(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<bool, String> {
    Ok(watchdog.get_auto_restart().await)
}

#[tauri::command]
pub async fn test_group_delay(group_name: String) -> Result<String, String> {
    mihomo::test_group_delay(&group_name)
        .await
        .map_err(|e| format!("Failed to test group delay: {}", e))
        .map(|_| "Delay test completed".to_string())
}

#[tauri::command]
pub async fn get_current_ip() -> Result<serde_json::Value, String> {
    let proxy_result = get_ip_via_proxy().await;

    if let Ok(data) = proxy_result {
        tracing::info!("通过代理成功获取IP信息: {:?}", data);
        return Ok(data);
    }

    tracing::warn!("通过代理获取IP失败，尝试直连...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let services = vec![
        "http://ip-api.com/json/",
        "https://ipapi.co/json/",
        "https://api.ip.sb/geoip",
    ];

    for service in services {
        tracing::debug!("尝试从 {} 直连获取IP信息", service);
        if let Ok(response) = client.get(service).send().await {
            if let Ok(mut data) = response.json::<serde_json::Value>().await {
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("proxy_status".to_string(), serde_json::json!("direct"));
                }
                tracing::info!("直连获取IP信息: {:?}", data);
                return Ok(data);
            }
        }
    }

    Err("Failed to get IP information from all services".to_string())
}

async fn get_ip_via_proxy() -> Result<serde_json::Value, String> {
    let config = crate::config::load_config()
        .await
        .map_err(|e| format!("Failed to load config: {}", e))?;

    let http_port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(7890) as u16;
    let proxy_url = format!("http://127.0.0.1:{}", http_port);

    let proxy =
        reqwest::Proxy::all(&proxy_url).map_err(|e| format!("Failed to create proxy: {}", e))?;

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client with proxy: {}", e))?;

    let services = vec![
        "http://ip-api.com/json/",
        "https://ipapi.co/json/",
        "https://api.ip.sb/geoip",
    ];

    for service in services {
        tracing::debug!("尝试通过代理从 {} 获取IP信息", service);
        match client.get(service).send().await {
            Ok(response) => {
                if let Ok(mut data) = response.json::<serde_json::Value>().await {
                    if let Some(obj) = data.as_object_mut() {
                        obj.insert("proxy_status".to_string(), serde_json::json!("proxied"));
                    }
                    return Ok(data);
                }
            }
            Err(e) => {
                tracing::debug!("代理请求 {} 失败: {}", service, e);
            }
        }
    }

    Err("Failed to get IP via proxy".to_string())
}

#[tauri::command]
pub async fn test_all_proxies(
    _test_url: Option<String>,
    _timeout: Option<u32>,
) -> Result<serde_json::Value, String> {
    mihomo::test_all_groups_delay()
        .await
        .map_err(|e| format!("Failed to test all proxies: {}", e))
}

#[tauri::command]
pub async fn validate_config(
    config: serde_json::Value,
) -> Result<validator::ValidationResult, String> {
    validator::validate_config(&config)
        .await
        .map_err(|e| format!("Failed to validate config: {}", e))
}

#[tauri::command]
pub async fn list_config_backups() -> Result<Vec<String>, String> {
    backup::list_backups()
        .await
        .map_err(|e| format!("Failed to list backups: {}", e))
}

#[tauri::command]
pub async fn restore_config_backup(
    backup_filename: String,
) -> Result<backup::RestoreResult, String> {
    backup::restore_config(&backup_filename)
        .await
        .map_err(|e| format!("Failed to restore backup: {}", e))
}

#[tauri::command]
pub async fn delete_config_backup(backup_filename: String) -> Result<String, String> {
    backup::delete_backup(&backup_filename)
        .await
        .map_err(|e| format!("Failed to delete backup: {}", e))
        .map(|_| "备份已删除".to_string())
}

#[tauri::command]
pub async fn rename_config_backup(
    old_filename: String,
    new_label: String,
) -> Result<String, String> {
    backup::rename_backup(&old_filename, &new_label)
        .await
        .map_err(|e| format!("Failed to rename backup: {}", e))
}

#[tauri::command]
pub async fn check_mihomo_binary() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let mihomo_paths = vec![
        "mihomo.exe",
        "clash-meta.exe",
        "./mihomo.exe",
        "./clash-meta.exe",
    ];

    #[cfg(not(target_os = "windows"))]
    let mihomo_paths = vec![
        "mihomo",
        "clash-meta",
        "./mihomo",
        "./clash-meta",
        "/usr/bin/mihomo",
        "/usr/local/bin/mihomo",
    ];

    for path in mihomo_paths {
        if let Ok(output) = std::process::Command::new(path).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(format!("找到 mihomo: {} - {}", path, version.trim()));
            }
        }
    }

    Err(
        "未找到 mihomo 二进制文件。请从 https://github.com/MetaCubeX/mihomo/releases 下载并安装。"
            .to_string(),
    )
}

#[tauri::command]
pub async fn check_admin_privileges() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let output = Command::new("powershell")
            .args(["-Command", "([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)"])
            .output()
            .map_err(|e| format!("检查权限失败: {}", e))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(result.trim().eq_ignore_ascii_case("true"))
        } else {
            Ok(false)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(unsafe { libc::geteuid() } == 0)
    }
}

#[tauri::command]
pub async fn restart_as_admin() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        let result = Command::new("powershell")
            .args([
                "-Command",
                &format!("Start-Process '{}' -Verb RunAs", current_exe.display()),
            ])
            .spawn();

        match result {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => Err(format!("以管理员身份重启失败: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        let display = std::env::var("DISPLAY").unwrap_or(":0".to_string());
        let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| {
            format!("{}/.Xauthority", std::env::var("HOME").unwrap_or_default())
        });
        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

        let launch_cmd = format!(
            "DISPLAY={} XAUTHORITY={} WAYLAND_DISPLAY={} XDG_SESSION_TYPE={} GDK_BACKEND=x11,wayland {}",
            display, xauth, wayland_display, session_type, current_exe.display()
        );

        let result = Command::new("bash")
            .arg("-c")
            .arg(format!("pkexec bash -c '{}' &", launch_cmd))
            .spawn();

        match result {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_millis(200));
                std::process::exit(0);
            }
            Err(e) => Err(format!("启动失败: {}", e)),
        }
    }
}

#[tauri::command]
pub async fn get_bundled_mihomo_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let mihomo_binary = "mihomo.exe";
    #[cfg(not(target_os = "windows"))]
    let mihomo_binary = "mihomo";

    #[cfg(target_os = "windows")]
    let system_paths: Vec<&str> = vec![];
    #[cfg(not(target_os = "windows"))]
    let system_paths = vec![
        "/usr/local/bin/mihomo",
        "/usr/bin/mihomo",
        "/opt/mihomo/mihomo",
    ];

    for path in &system_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    let app_dir = std::env::current_exe()
        .map_err(|e| format!("获取应用目录失败: {}", e))?
        .parent()
        .ok_or("无法获取应用目录")?
        .to_path_buf();

    let mihomo_path = app_dir.join(mihomo_binary);

    if mihomo_path.exists() {
        Ok(mihomo_path.to_string_lossy().to_string())
    } else {
        Err(format!(
            "未找到 {} 文件。请确保 mihomo 已安装在系统路径或应用目录中",
            mihomo_binary
        ))
    }
}

#[tauri::command]
pub async fn backup_subscriptions() -> Result<String, String> {
    crate::subscription::backup_subscriptions()
        .await
        .map_err(|e| format!("备份订阅链接失败: {}", e))
}
