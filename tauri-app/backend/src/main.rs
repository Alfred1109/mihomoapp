// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod config;
mod config_manager;
mod error;
mod events;
mod mihomo;
mod platform_config;
mod subscription;
mod validator;
mod watchdog;

use serde::{Deserialize, Serialize};
use tauri::{
    CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

lazy_static::lazy_static! {
    // 防止并发启动的全局锁
    static ref SERVICE_START_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub mihomo_running: bool,
    pub mihomo_process: Option<u32>,
}

fn resolve_app_dir() -> Result<std::path::PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| format!("获取应用目录失败: {}", e))?
        .parent()
        .ok_or("无法获取应用目录".to_string())
        .map(|p| p.to_path_buf())
}

fn resolve_winsw_source(app_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let direct = app_dir.join("winsw.exe");
    if direct.exists() {
        return Ok(direct);
    }
    let resource = app_dir.join("resources").join("winsw.exe");
    if resource.exists() {
        return Ok(resource);
    }
    Err(format!(
        "未找到 winsw.exe 文件。期望位置: {}",
        direct.display()
    ))
}

fn resolve_mihomo_path(app_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let mihomo = app_dir.join("mihomo.exe");
    if mihomo.exists() {
        return Ok(mihomo);
    }

    // 尝试从 resources 目录复制到应用目录
    let resource = app_dir.join("resources").join("mihomo.exe");
    if resource.exists() {
        std::fs::copy(&resource, &mihomo).map_err(|e| format!("复制 mihomo.exe 失败: {}", e))?;
        if mihomo.exists() {
            return Ok(mihomo);
        }
    }

    Err(format!(
        "未找到 mihomo.exe 文件。期望位置: {}",
        mihomo.display()
    ))
}

fn ensure_winsw_files(
    app_dir: &std::path::Path,
    winsw_source: &std::path::Path,
    mihomo_path: &std::path::Path,
    config_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use std::fs;

    let winsw_exe = app_dir.join("MihomoService.exe");
    let winsw_xml = app_dir.join("MihomoService.xml");

    if !winsw_exe.exists() {
        fs::copy(winsw_source, &winsw_exe).map_err(|e| format!("复制 WinSW 失败: {}", e))?;
    }

    let xml_content = format!(
        r#"<service>
  <id>MihomoService</id>
  <name>Mihomo Proxy Service</name>
  <description>Mihomo Proxy Service</description>
  <executable>{}</executable>
  <arguments>-f "{}"</arguments>
  <logpath>{}</logpath>
  <log mode="roll" />
</service>"#,
        mihomo_path.display(),
        config_path.display(),
        app_dir.display()
    );

    fs::write(&winsw_xml, xml_content).map_err(|e| format!("写入 WinSW 配置失败: {}", e))?;

    Ok(winsw_exe)
}


#[allow(dead_code)]
#[tauri::command]
async fn get_bundled_winsw_path() -> Result<String, String> {
    // 获取应用程序目录
    let app_dir = std::env::current_exe()
        .map_err(|e| format!("获取应用目录失败: {}", e))?
        .parent()
        .ok_or("无法获取应用目录")?
        .to_path_buf();

    // 简单方案：直接使用应用目录下的 winsw.exe
    let winsw_path = app_dir.join("winsw.exe");

    if winsw_path.exists() {
        return Ok(winsw_path.to_string_lossy().to_string());
    }

    // 备用：resources 目录
    let resource_path = app_dir.join("resources").join("winsw.exe");
    if resource_path.exists() {
        return Ok(resource_path.to_string_lossy().to_string());
    }

    Err(format!(
        "未找到 winsw.exe 文件。期望位置: {}",
        winsw_path.display()
    ))
}

type AppStateType = tokio::sync::RwLock<AppState>;

#[tauri::command]
async fn get_mihomo_status(state: State<'_, AppStateType>) -> Result<bool, String> {
    let app_state = state.read().await;
    Ok(app_state.mihomo_running)
}

#[tauri::command]
async fn start_mihomo_service(
    state: State<'_, AppStateType>,
    app: tauri::AppHandle,
    watchdog: State<'_, std::sync::Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    // 防止并发启动 - 获取启动锁
    let _start_lock = SERVICE_START_LOCK.lock().await;
    
    match mihomo::start_mihomo().await {
        Ok(process_id) => {
            {
                let mut app_state = state.write().await;
                app_state.mihomo_running = true;
                app_state.mihomo_process = Some(process_id);
            }

            // 更新 watchdog 跟踪的进程
            watchdog.set_process(process_id).await;

            events::emit_mihomo_status(
                &app,
                events::MihomoStatusEvent {
                    running: true,
                    process_id: Some(process_id),
                    timestamp: events::get_current_timestamp(),
                },
            );

            Ok("Mihomo service started successfully".to_string())
        }
        Err(e) => Err(format!("Failed to start mihomo: {}", e)),
    }
}

#[tauri::command]
async fn stop_mihomo_service(
    state: State<'_, AppStateType>,
    app: tauri::AppHandle,
    watchdog: State<'_, std::sync::Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    match mihomo::stop_mihomo().await {
        Ok(_) => {
            {
                let mut app_state = state.write().await;
                app_state.mihomo_running = false;
                app_state.mihomo_process = None;
            }

            // 清除 watchdog 跟踪（会自动禁用自动重启，防止竞态条件）
            watchdog.clear_process().await;

            // 等待一小段时间确保服务完全停止
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            events::emit_mihomo_status(
                &app,
                events::MihomoStatusEvent {
                    running: false,
                    process_id: None,
                    timestamp: events::get_current_timestamp(),
                },
            );

            Ok("Mihomo service stopped successfully".to_string())
        }
        Err(e) => Err(format!("Failed to stop mihomo: {}", e)),
    }
}

#[tauri::command]
async fn get_mihomo_config() -> Result<serde_json::Value, String> {
    config::load_config()
        .await
        .map_err(|e| format!("Failed to load config: {}", e))
}

#[tauri::command]
async fn save_mihomo_config(
    config: serde_json::Value,
    app: tauri::AppHandle,
) -> Result<String, String> {
    match config::save_config(config).await {
        Ok(_) => {
            events::emit_config_change(
                &app,
                events::ConfigChangeEvent {
                    config_path: config::get_config_path()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    timestamp: events::get_current_timestamp(),
                },
            );
            Ok("Configuration saved successfully".to_string())
        }
        Err(e) => Err(format!("Failed to save config: {}", e)),
    }
}

#[tauri::command]
async fn get_proxies() -> Result<serde_json::Value, String> {
    mihomo::get_proxies()
        .await
        .map_err(|e| format!("Failed to get proxies: {}", e))
}

#[tauri::command]
async fn switch_proxy(
    group_name: String,
    proxy_name: String,
    app: tauri::AppHandle,
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
async fn add_subscription(
    name: String,
    url: String,
    user_agent: Option<String>,
    use_proxy: bool,
) -> Result<String, String> {
    subscription::add_subscription(name, url, user_agent, use_proxy)
        .await
        .map_err(|e| format!("Failed to add subscription: {}", e))
        .map(|_| "Subscription added successfully".to_string())
}

#[tauri::command]
async fn get_subscriptions() -> Result<Vec<subscription::Subscription>, String> {
    subscription::get_subscriptions()
        .await
        .map_err(|e| format!("Failed to get subscriptions: {}", e))
}

#[tauri::command]
async fn update_subscription(id: String) -> Result<String, String> {
    subscription::update_subscription(&id)
        .await
        .map_err(|e| format!("Failed to update subscription: {}", e))
        .map(|_| "Subscription updated successfully".to_string())
}

#[tauri::command]
async fn delete_subscription(id: String) -> Result<String, String> {
    subscription::delete_subscription(&id)
        .await
        .map_err(|e| format!("Failed to delete subscription: {}", e))
        .map(|_| "Subscription deleted successfully".to_string())
}

#[tauri::command]
async fn generate_config_from_subscriptions(
    subscription_ids: Vec<String>,
) -> Result<String, String> {
    subscription::generate_config_from_subscriptions(subscription_ids)
        .await
        .map_err(|e| format!("Failed to generate config: {}", e))
        .map(|_| "Configuration generated successfully".to_string())
}

#[tauri::command]
async fn enable_tun_mode(enable: bool) -> Result<String, String> {
    config::set_tun_mode(enable)
        .await
        .map_err(|e| format!("Failed to set TUN mode: {}", e))
        .map(|_| {
            if enable {
                "TUN mode enabled"
            } else {
                "TUN mode disabled"
            }
            .to_string()
        })
}

#[tauri::command]
async fn set_auto_restart(
    enabled: bool,
    watchdog: State<'_, std::sync::Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    watchdog.set_auto_restart(enabled).await;
    Ok(format!(
        "Auto-restart {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}

#[tauri::command]
async fn get_auto_restart(
    watchdog: State<'_, std::sync::Arc<watchdog::ProcessWatchdog>>,
) -> Result<bool, String> {
    Ok(watchdog.get_auto_restart().await)
}

#[tauri::command]
async fn test_group_delay(group_name: String) -> Result<String, String> {
    mihomo::test_group_delay(&group_name)
        .await
        .map_err(|e| format!("Failed to test group delay: {}", e))
        .map(|_| "Delay test completed".to_string())
}

#[tauri::command]
async fn get_current_ip() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // 尝试多个IP查询服务，优先使用返回完整信息的服务
    let services = vec![
        "http://ip-api.com/json/", // 免费，返回完整地理位置信息
        "https://ipapi.co/json/",
        "https://api.ip.sb/geoip",
    ];

    for service in services {
        tracing::debug!("尝试从 {} 获取IP信息", service);
        if let Ok(response) = client.get(service).send().await {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                tracing::info!("成功获取IP信息: {:?}", data);
                return Ok(data);
            }
        }
    }

    Err("Failed to get IP information from all services".to_string())
}

#[tauri::command]
async fn test_all_proxies(
    _test_url: Option<String>,
    _timeout: Option<u32>,
) -> Result<serde_json::Value, String> {
    // 使用 test_all_groups_delay 来测速，这样会更新 Mihomo 内部状态
    // test_url 和 timeout 参数暂时忽略，使用默认值
    mihomo::test_all_groups_delay()
        .await
        .map_err(|e| format!("Failed to test all proxies: {}", e))
}

#[tauri::command]
async fn validate_config(config: serde_json::Value) -> Result<validator::ValidationResult, String> {
    validator::validate_config(&config)
        .await
        .map_err(|e| format!("Failed to validate config: {}", e))
}

#[tauri::command]
async fn list_config_backups() -> Result<Vec<String>, String> {
    backup::list_backups()
        .await
        .map_err(|e| format!("Failed to list backups: {}", e))
}

#[tauri::command]
async fn restore_config_backup(backup_filename: String) -> Result<backup::RestoreResult, String> {
    backup::restore_config(&backup_filename)
        .await
        .map_err(|e| format!("Failed to restore backup: {}", e))
}

#[tauri::command]
async fn delete_config_backup(backup_filename: String) -> Result<String, String> {
    backup::delete_backup(&backup_filename)
        .await
        .map_err(|e| format!("Failed to delete backup: {}", e))
        .map(|_| "备份已删除".to_string())
}

#[tauri::command]
async fn rename_config_backup(old_filename: String, new_label: String) -> Result<String, String> {
    backup::rename_backup(&old_filename, &new_label)
        .await
        .map_err(|e| format!("Failed to rename backup: {}", e))
}

#[tauri::command]
async fn check_mihomo_binary() -> Result<String, String> {
    // 根据平台检查不同的可执行文件
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
async fn check_admin_privileges() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // 使用 PowerShell 检查管理员权限
        let output = Command::new("powershell")
            .args(["-Command", "([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)"])
            .output()
            .map_err(|e| format!("检查权限失败: {}", e))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            let trimmed = result.trim();
            Ok(trimmed.eq_ignore_ascii_case("true"))
        } else {
            Ok(false)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 在非 Windows 系统上，检查是否为 root 用户
        Ok(unsafe { libc::geteuid() } == 0)
    }
}

#[tauri::command]
async fn restart_as_admin() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // 获取当前可执行文件路径
        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        // 使用 PowerShell 以管理员身份重启应用
        let result = Command::new("powershell")
            .args([
                "-Command",
                &format!("Start-Process '{}' -Verb RunAs", current_exe.display()),
            ])
            .spawn();

        match result {
            Ok(_) => {
                // 启动成功，退出当前进程
                std::process::exit(0);
            }
            Err(e) => Err(format!("以管理员身份重启失败: {}", e)),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        // 获取当前可执行文件路径
        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        // 获取显示环境变量
        let display = std::env::var("DISPLAY").unwrap_or(":0".to_string());
        let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| {
            format!("{}/.Xauthority", std::env::var("HOME").unwrap_or_default())
        });
        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

        // 构建启动命令（不包含 &）
        let launch_cmd = format!(
            "DISPLAY={} XAUTHORITY={} WAYLAND_DISPLAY={} XDG_SESSION_TYPE={} GDK_BACKEND=x11,wayland {}",
            display, xauth, wayland_display, session_type, current_exe.display()
        );

        // 使用 pkexec 启动新进程，整个 pkexec 在后台运行
        let result = Command::new("bash")
            .arg("-c")
            .arg(format!("pkexec bash -c '{}' &", launch_cmd))
            .spawn();

        match result {
            Ok(_) => {
                // 短暂延迟后退出，确保 bash/pkexec 进程已启动
                std::thread::sleep(std::time::Duration::from_millis(200));
                std::process::exit(0);
            }
            Err(e) => Err(format!("启动失败: {}", e)),
        }
    }
}

#[tauri::command]
async fn get_bundled_mihomo_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let mihomo_binary = "mihomo.exe";
    #[cfg(not(target_os = "windows"))]
    let mihomo_binary = "mihomo";

    // 系统路径列表
    #[cfg(target_os = "windows")]
    let system_paths: Vec<&str> = vec![];
    #[cfg(not(target_os = "windows"))]
    let system_paths = vec![
        "/usr/local/bin/mihomo",
        "/usr/bin/mihomo",
        "/opt/mihomo/mihomo",
    ];

    // 首先检查系统路径
    for path in &system_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // 如果系统路径找不到，检查应用目录
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
async fn install_mihomo_service() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_dir = resolve_app_dir()?;
        let winsw_source = resolve_winsw_source(&app_dir)?;
        let mihomo_path = resolve_mihomo_path(&app_dir)?;

        // 创建配置目录
        let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?.join("mihomo");

        std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建配置目录失败: {}", e))?;

        let config_path = config_dir.join("config.yaml");

        // 如果配置文件不存在，创建默认配置
        if !config_path.exists() {
            crate::config::save_config(crate::mihomo::create_default_config())
                .await
                .map_err(|e| format!("创建默认配置失败: {}", e))?;
        }

        let winsw_exe = ensure_winsw_files(&app_dir, &winsw_source, &mihomo_path, &config_path)?;

        // 先尝试卸载旧服务（忽略错误）
        let _ = Command::new(&winsw_exe).arg("stop").output();
        let _ = Command::new(&winsw_exe).arg("uninstall").output();

        // 安装服务
        let output = Command::new(&winsw_exe)
            .arg("install")
            .output()
            .map_err(|e| format!("安装服务失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务安装成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "服务安装失败:\n标准输出: {}\n错误输出: {}",
                stdout, stderr
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        use std::path::Path;
        use std::process::Command;

        // 创建配置目录
        let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?.join("mihomo");

        fs::create_dir_all(&config_dir).map_err(|e| format!("创建配置目录失败: {}", e))?;

        let config_path = config_dir.join("config.yaml");

        // 如果配置文件不存在，创建默认配置
        if !config_path.exists() {
            crate::config::save_config(crate::mihomo::create_default_config())
                .await
                .map_err(|e| format!("创建默认配置失败: {}", e))?;
        }

        // 检查 mihomo 二进制文件是否存在
        let mihomo_path = "/usr/local/bin/mihomo";
        if !Path::new(mihomo_path).exists() {
            // 尝试从系统PATH中找到mihomo
            match Command::new("which").arg("mihomo").output() {
                Ok(output) if output.status.success() => {
                    // mihomo exists in PATH
                }
                _ => {
                    return Err("未找到 mihomo 二进制文件，请先安装 mihomo 到 /usr/local/bin/mihomo 或系统PATH中".to_string());
                }
            }
        }

        // 确定 mihomo 二进制文件路径
        let mihomo_binary = if Path::new("/usr/local/bin/mihomo").exists() {
            "/usr/local/bin/mihomo".to_string()
        } else if Path::new("/usr/bin/mihomo").exists() {
            "/usr/bin/mihomo".to_string()
        } else {
            // 尝试从PATH中找到
            match Command::new("which").arg("mihomo").output() {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                }
                _ => "/usr/local/bin/mihomo".to_string(), // 默认路径
            }
        };

        // 创建 systemd 服务文件内容
        let service_content = format!(
            r#"[Unit]
Description=Mihomo (Clash Meta) Proxy Service
After=network.target

[Service]
Type=simple
ExecStart={} -d {}
Restart=always
RestartSec=3
User=root
Group=root

[Install]
WantedBy=multi-user.target
"#,
            mihomo_binary,
            config_path
                .parent()
                .ok_or_else(|| "Failed to get config directory".to_string())?
                .display()
        );

        // 写入 systemd 服务文件
        let service_file = "/etc/systemd/system/mihomo.service";
        fs::write(service_file, service_content)
            .map_err(|e| format!("写入服务文件失败 (需要 root 权限): {}", e))?;

        // Linux上不需要创建特定用户，使用root运行以获得必要权限

        // 重新加载 systemd
        let output = Command::new("systemctl")
            .arg("daemon-reload")
            .output()
            .map_err(|e| format!("重新加载 systemd 失败: {}", e))?;

        if !output.status.success() {
            return Err("重新加载 systemd 失败".to_string());
        }

        // 启用服务
        let output = Command::new("systemctl")
            .args(["enable", "mihomo.service"])
            .output()
            .map_err(|e| format!("启用服务失败: {}", e))?;

        if output.status.success() {
            Ok("Mihomo 服务安装并启用成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("启用服务失败: {}", stderr))
        }
    }
}

#[tauri::command]
async fn start_mihomo_service_cmd() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_dir = resolve_app_dir()?;
        let winsw_source = resolve_winsw_source(&app_dir)?;
        let mihomo_path = resolve_mihomo_path(&app_dir)?;
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo")
            .join("config.yaml");
        let winsw_exe = ensure_winsw_files(&app_dir, &winsw_source, &mihomo_path, &config_path)?;

        // 首先检查服务是否存在
        let query_output = Command::new("sc")
            .args(["query", "MihomoService"])
            .output()
            .map_err(|e| format!("查询服务失败: {}", e))?;

        if !query_output.status.success() {
            return Err("MihomoService 服务不存在，请先安装服务".to_string());
        }

        // 尝试启动服务
        let output = Command::new(&winsw_exe)
            .arg("start")
            .output()
            .map_err(|e| format!("启动服务命令执行失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务启动成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "服务启动失败:\n标准输出: {}\n错误输出: {}",
                stdout, stderr
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        // 检查服务是否存在
        let check_output = Command::new("systemctl")
            .args(["list-unit-files", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务失败: {}", e))?;

        if !String::from_utf8_lossy(&check_output.stdout).contains("mihomo.service") {
            return Err("Mihomo 服务未安装，请先安装服务".to_string());
        }

        // 启动服务（使用 pkexec 获取 root 权限）
        let output = Command::new("pkexec")
            .args(["systemctl", "start", "mihomo.service"])
            .output()
            .map_err(|e| format!("启动服务失败: {}", e))?;

        if output.status.success() {
            Ok("Mihomo 服务启动成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("服务启动失败: {}", stderr))
        }
    }
}

#[tauri::command]
async fn stop_mihomo_service_cmd() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_dir = resolve_app_dir()?;
        let winsw_source = resolve_winsw_source(&app_dir)?;
        let mihomo_path = resolve_mihomo_path(&app_dir)?;
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo")
            .join("config.yaml");
        let winsw_exe = ensure_winsw_files(&app_dir, &winsw_source, &mihomo_path, &config_path)?;

        let output = Command::new(&winsw_exe)
            .arg("stop")
            .output()
            .map_err(|e| format!("停止服务失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务停止成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "服务停止失败:\n标准输出: {}\n错误输出: {}",
                stdout, stderr
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        // 停止服务（使用 pkexec 获取 root 权限）
        let output = Command::new("pkexec")
            .args(["systemctl", "stop", "mihomo.service"])
            .output()
            .map_err(|e| format!("停止服务失败: {}", e))?;

        if output.status.success() {
            Ok("Mihomo 服务停止成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("服务停止失败: {}", stderr))
        }
    }
}

#[tauri::command]
async fn restart_mihomo_service_cmd() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_dir = resolve_app_dir()?;
        let winsw_source = resolve_winsw_source(&app_dir)?;
        let mihomo_path = resolve_mihomo_path(&app_dir)?;
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo")
            .join("config.yaml");
        let winsw_exe = ensure_winsw_files(&app_dir, &winsw_source, &mihomo_path, &config_path)?;

        // WinSW 支持 restart 命令
        let output = Command::new(&winsw_exe)
            .arg("restart")
            .output()
            .map_err(|e| format!("重启服务失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务重启成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "服务重启失败:\n标准输出: {}\n错误输出: {}",
                stdout, stderr
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        // 重启服务（使用 pkexec 获取 root 权限）
        let output = Command::new("pkexec")
            .args(["systemctl", "restart", "mihomo.service"])
            .output()
            .map_err(|e| format!("重启服务失败: {}", e))?;

        if output.status.success() {
            Ok("Mihomo 服务重启成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("服务重启失败: {}", stderr))
        }
    }
}

#[tauri::command]
async fn uninstall_mihomo_service() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_dir = resolve_app_dir()?;
        let winsw_source = resolve_winsw_source(&app_dir)?;
        let mihomo_path = resolve_mihomo_path(&app_dir)?;
        let config_path = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo")
            .join("config.yaml");
        let winsw_exe = ensure_winsw_files(&app_dir, &winsw_source, &mihomo_path, &config_path)?;

        // 先停止服务
        let _ = Command::new(&winsw_exe).arg("stop").output();

        // 删除服务
        let output = Command::new(&winsw_exe)
            .arg("uninstall")
            .output()
            .map_err(|e| format!("卸载服务失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务卸载成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            Err(format!(
                "服务卸载失败:\n标准输出: {}\n错误输出: {}",
                stdout, stderr
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        use std::process::Command;

        // 停止并禁用服务
        let _ = Command::new("systemctl")
            .args(["stop", "mihomo.service"])
            .output();

        let _ = Command::new("systemctl")
            .args(["disable", "mihomo.service"])
            .output();

        // 删除服务文件
        match fs::remove_file("/etc/systemd/system/mihomo.service") {
            Ok(_) => {
                // 重新加载 systemd
                let _ = Command::new("systemctl").arg("daemon-reload").output();

                Ok("Mihomo 服务卸载成功".to_string())
            }
            Err(e) => Err(format!("删除服务文件失败 (需要 root 权限): {}", e)),
        }
    }
}

#[tauri::command]
async fn set_autostart(enable: bool) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        let app_name = "MihomoManager";
        let reg_key = r"Software\Microsoft\Windows\CurrentVersion\Run";

        if enable {
            // 添加到注册表
            let output = Command::new("reg")
                .args([
                    "add",
                    &format!("HKCU\\{}", reg_key),
                    "/v",
                    app_name,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\"", current_exe.display()),
                    "/f",
                ])
                .output()
                .map_err(|e| format!("设置开机自启失败: {}", e))?;

            if output.status.success() {
                Ok("已启用开机自启".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("设置开机自启失败: {}", stderr))
            }
        } else {
            // 从注册表删除
            let output = Command::new("reg")
                .args([
                    "delete",
                    &format!("HKCU\\{}", reg_key),
                    "/v",
                    app_name,
                    "/f",
                ])
                .output()
                .map_err(|e| format!("取消开机自启失败: {}", e))?;

            if output.status.success()
                || String::from_utf8_lossy(&output.stderr).contains("无法找到")
            {
                Ok("已取消开机自启".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("取消开机自启失败: {}", stderr))
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        use std::path::Path;

        let home_dir = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录".to_string())?;

        let autostart_dir = Path::new(&home_dir).join(".config/autostart");
        let desktop_file = autostart_dir.join("mihomo-manager.desktop");

        if enable {
            // 创建 autostart 目录
            fs::create_dir_all(&autostart_dir)
                .map_err(|e| format!("创建 autostart 目录失败: {}", e))?;

            let current_exe =
                std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

            // 创建 .desktop 文件
            let desktop_content = format!(
                "[Desktop Entry]\n\
                Type=Application\n\
                Name=Mihomo Manager\n\
                Exec={}\n\
                Hidden=false\n\
                NoDisplay=false\n\
                X-GNOME-Autostart-enabled=true\n",
                current_exe.display()
            );

            fs::write(&desktop_file, desktop_content)
                .map_err(|e| format!("写入 desktop 文件失败: {}", e))?;

            Ok("已启用开机自启".to_string())
        } else {
            // 删除 .desktop 文件
            if desktop_file.exists() {
                fs::remove_file(&desktop_file)
                    .map_err(|e| format!("删除 desktop 文件失败: {}", e))?;
            }
            Ok("已取消开机自启".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("当前平台不支持开机自启功能".to_string())
    }
}

#[tauri::command]
async fn get_autostart_status() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let app_name = "MihomoManager";
        let reg_key = r"Software\Microsoft\Windows\CurrentVersion\Run";

        let output = Command::new("reg")
            .args(["query", &format!("HKCU\\{}", reg_key), "/v", app_name])
            .output()
            .map_err(|e| format!("查询开机自启状态失败: {}", e))?;

        Ok(output.status.success())
    }

    #[cfg(target_os = "linux")]
    {
        use std::path::Path;

        let home_dir = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录".to_string())?;

        let desktop_file = Path::new(&home_dir).join(".config/autostart/mihomo-manager.desktop");

        Ok(desktop_file.exists())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Ok(false)
    }
}

#[tauri::command]
async fn set_silent_start(enable: bool) -> Result<String, String> {
    // 保存静默启动设置到配置文件
    let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?;

    let app_config_dir = config_dir.join("mihomo-manager");
    std::fs::create_dir_all(&app_config_dir).map_err(|e| format!("创建配置目录失败: {}", e))?;

    let settings_file = app_config_dir.join("settings.json");

    let settings = serde_json::json!({
        "silent_start": enable
    });

    let settings_json =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("序列化设置失败: {}", e))?;
    std::fs::write(&settings_file, settings_json).map_err(|e| format!("保存设置失败: {}", e))?;

    if enable {
        Ok("已启用静默启动".to_string())
    } else {
        Ok("已取消静默启动".to_string())
    }
}

#[tauri::command]
async fn get_silent_start_status() -> Result<bool, String> {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir,
        None => return Ok(false),
    };

    let settings_file = config_dir.join("mihomo-manager/settings.json");

    if !settings_file.exists() {
        return Ok(false);
    }

    let content =
        std::fs::read_to_string(&settings_file).map_err(|e| format!("读取设置失败: {}", e))?;

    let settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析设置失败: {}", e))?;

    Ok(settings["silent_start"].as_bool().unwrap_or(false))
}

#[tauri::command]
async fn get_mihomo_service_status() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let output = Command::new("sc")
            .args(["query", "MihomoService"])
            .output()
            .map_err(|e| format!("查询服务状态失败: {}", e))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("RUNNING") {
            Ok("running".to_string())
        } else if output_str.contains("STOPPED") {
            Ok("stopped".to_string())
        } else if output.status.success() {
            Ok("installed".to_string())
        } else {
            Ok("not_installed".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        // 检查服务是否安装
        let unit_files_output = Command::new("systemctl")
            .args(["list-unit-files", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务安装状态失败: {}", e))?;

        if !String::from_utf8_lossy(&unit_files_output.stdout).contains("mihomo.service") {
            return Ok("not_installed".to_string());
        }

        // 检查服务状态
        let status_output = Command::new("systemctl")
            .args(["is-active", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务状态失败: {}", e))?;

        let status_str = String::from_utf8_lossy(&status_output.stdout);
        let status = status_str.trim();

        match status {
            "active" => Ok("running".to_string()),
            "activating" => Ok("running".to_string()), // 正在启动，视为运行中
            "reloading" => Ok("running".to_string()),  // 重新加载，视为运行中
            "inactive" => Ok("stopped".to_string()),
            "deactivating" => Ok("stopped".to_string()), // 正在停止，视为已停止
            "failed" => Ok("stopped".to_string()),
            _ => Ok("installed".to_string()),
        }
    }
}

fn main() {
    // 检测是否以 root 运行，如果是则禁用 WebView 沙箱
    #[cfg(target_os = "linux")]
    {
        unsafe {
            if libc::geteuid() == 0 {
                // 以 root 运行时，必须禁用沙箱
                std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

    // 创建系统托盘菜单
    let show = CustomMenuItem::new("show".to_string(), "显示窗口");
    let hide = CustomMenuItem::new("hide".to_string(), "隐藏窗口");
    let start_service = CustomMenuItem::new("start".to_string(), "启动服务");
    let stop_service = CustomMenuItem::new("stop".to_string(), "停止服务");
    let quit = CustomMenuItem::new("quit".to_string(), "退出");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(start_service)
        .add_item(stop_service)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppStateType::new(AppState::default()))
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                if let Some(window) = app.get_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "show" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "start" => {
                        // 启动服务
                        tauri::async_runtime::spawn(async move {
                            let _ = mihomo::start_mihomo().await;
                        });
                    }
                    "stop" => {
                        // 停止服务
                        tauri::async_runtime::spawn(async move {
                            let _ = mihomo::stop_mihomo().await;
                        });
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                // 关闭窗口时最小化到托盘而不是退出
                let _ = event.window().hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_mihomo_status,
            start_mihomo_service,
            stop_mihomo_service,
            get_mihomo_config,
            save_mihomo_config,
            get_proxies,
            switch_proxy,
            add_subscription,
            get_subscriptions,
            update_subscription,
            delete_subscription,
            generate_config_from_subscriptions,
            enable_tun_mode,
            set_auto_restart,
            get_auto_restart,
            test_group_delay,
            test_all_proxies,
            validate_config,
            list_config_backups,
            restore_config_backup,
            delete_config_backup,
            rename_config_backup,
            get_current_ip,
            check_mihomo_binary,
            check_admin_privileges,
            restart_as_admin,
            get_bundled_mihomo_path,
            install_mihomo_service,
            start_mihomo_service_cmd,
            stop_mihomo_service_cmd,
            restart_mihomo_service_cmd,
            uninstall_mihomo_service,
            get_mihomo_service_status,
            set_autostart,
            get_autostart_status,
            set_silent_start,
            get_silent_start_status
        ])
        .setup(|app| {
            // Initialize application
            let window = app
                .get_window("main")
                .ok_or("Failed to get main window")?;
            window.set_title("Mihomo Manager")?;

            // 初始化 ConfigManager
            let config_path = config::get_config_path().map_err(|e| e.to_string())?;
            tauri::async_runtime::block_on(async {
                config_manager::init_config_manager(config_path).await;
            });

            // 初始化 watchdog
            let watchdog = std::sync::Arc::new(watchdog::ProcessWatchdog::new(app.handle()));
            app.manage(watchdog.clone());

            // 启动 watchdog 监控
            let watchdog_clone = watchdog.clone();
            tauri::async_runtime::spawn(async move {
                watchdog_clone.start_monitoring().await;
            });

            // 检查是否启用静默启动
            let config_dir = dirs::config_dir();
            let mut silent_start = false;

            if let Some(dir) = config_dir {
                let settings_file = dir.join("mihomo-manager/settings.json");
                if settings_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&settings_file) {
                        if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                            silent_start = settings["silent_start"].as_bool().unwrap_or(false);
                        }
                    }
                }
            }

            if silent_start {
                // 静默启动：隐藏窗口，只显示托盘图标
                let _ = window.hide();
            } else {
                // 正常启动：显示窗口并获得焦点
                let _ = window.show();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| eprintln!("Error running tauri application: {}", e))
        .ok();
}
