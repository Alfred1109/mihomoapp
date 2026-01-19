// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mihomo;
mod config;
mod subscription;

use std::sync::Mutex;
use tauri::{Manager, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn resolve_winsw_source(app_dir: &std::path::PathBuf) -> Result<std::path::PathBuf, String> {
    let direct = app_dir.join("winsw.exe");
    if direct.exists() {
        return Ok(direct);
    }
    let resource = app_dir.join("resources").join("winsw.exe");
    if resource.exists() {
        return Ok(resource);
    }
    Err(format!("未找到 winsw.exe 文件。期望位置: {}", direct.display()))
}

fn resolve_mihomo_path(app_dir: &std::path::PathBuf) -> Result<std::path::PathBuf, String> {
    let mihomo = app_dir.join("mihomo.exe");
    if mihomo.exists() {
        return Ok(mihomo);
    }

    // 尝试从 resources 目录复制到应用目录
    let resource = app_dir.join("resources").join("mihomo.exe");
    if resource.exists() {
        std::fs::copy(&resource, &mihomo)
            .map_err(|e| format!("复制 mihomo.exe 失败: {}", e))?;
        if mihomo.exists() {
            return Ok(mihomo);
        }
    }

    Err(format!("未找到 mihomo.exe 文件。期望位置: {}", mihomo.display()))
}

fn ensure_winsw_files(
    app_dir: &std::path::PathBuf,
    winsw_source: &std::path::PathBuf,
    mihomo_path: &std::path::PathBuf,
    config_path: &std::path::PathBuf,
) -> Result<std::path::PathBuf, String> {
    use std::fs;

    let winsw_exe = app_dir.join("MihomoService.exe");
    let winsw_xml = app_dir.join("MihomoService.xml");

    if !winsw_exe.exists() {
        fs::copy(winsw_source, &winsw_exe)
            .map_err(|e| format!("复制 WinSW 失败: {}", e))?;
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

    fs::write(&winsw_xml, xml_content)
        .map_err(|e| format!("写入 WinSW 配置失败: {}", e))?;

    Ok(winsw_exe)
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mihomo_running: false,
            mihomo_process: None,
        }
    }
}

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

    Err(format!("未找到 winsw.exe 文件。期望位置: {}", winsw_path.display()))
}

type AppStateType = Mutex<AppState>;

#[tauri::command]
async fn get_mihomo_status(state: State<'_, AppStateType>) -> Result<bool, String> {
    let app_state = state.lock().unwrap();
    Ok(app_state.mihomo_running)
}

#[tauri::command]
async fn start_mihomo_service(state: State<'_, AppStateType>) -> Result<String, String> {
    match mihomo::start_mihomo().await {
        Ok(process_id) => {
            let mut app_state = state.lock().unwrap();
            app_state.mihomo_running = true;
            app_state.mihomo_process = Some(process_id);
            Ok("Mihomo service started successfully".to_string())
        }
        Err(e) => Err(format!("Failed to start mihomo: {}", e)),
    }
}

#[tauri::command]
async fn stop_mihomo_service(state: State<'_, AppStateType>) -> Result<String, String> {
    match mihomo::stop_mihomo().await {
        Ok(_) => {
            let mut app_state = state.lock().unwrap();
            app_state.mihomo_running = false;
            app_state.mihomo_process = None;
            Ok("Mihomo service stopped successfully".to_string())
        }
        Err(e) => Err(format!("Failed to stop mihomo: {}", e)),
    }
}

#[tauri::command]
async fn get_mihomo_config() -> Result<serde_json::Value, String> {
    config::load_config().await
        .map_err(|e| format!("Failed to load config: {}", e))
}

#[tauri::command]
async fn save_mihomo_config(config: serde_json::Value) -> Result<String, String> {
    config::save_config(config).await
        .map_err(|e| format!("Failed to save config: {}", e))
        .map(|_| "Configuration saved successfully".to_string())
}

#[tauri::command]
async fn get_proxies() -> Result<serde_json::Value, String> {
    mihomo::get_proxies().await
        .map_err(|e| format!("Failed to get proxies: {}", e))
}

#[tauri::command]
async fn switch_proxy(group_name: String, proxy_name: String) -> Result<String, String> {
    mihomo::switch_proxy(&group_name, &proxy_name).await
        .map_err(|e| format!("Failed to switch proxy: {}", e))
        .map(|_| "Proxy switched successfully".to_string())
}

#[tauri::command]
async fn add_subscription(name: String, url: String, user_agent: Option<String>) -> Result<String, String> {
    subscription::add_subscription(name, url, user_agent).await
        .map_err(|e| format!("Failed to add subscription: {}", e))
        .map(|_| "Subscription added successfully".to_string())
}

#[tauri::command]
async fn get_subscriptions() -> Result<Vec<subscription::Subscription>, String> {
    subscription::get_subscriptions().await
        .map_err(|e| format!("Failed to get subscriptions: {}", e))
}

#[tauri::command]
async fn update_subscription(id: String) -> Result<String, String> {
    subscription::update_subscription(&id).await
        .map_err(|e| format!("Failed to update subscription: {}", e))
        .map(|_| "Subscription updated successfully".to_string())
}

#[tauri::command]
async fn delete_subscription(id: String) -> Result<String, String> {
    subscription::delete_subscription(&id).await
        .map_err(|e| format!("Failed to delete subscription: {}", e))
        .map(|_| "Subscription deleted successfully".to_string())
}

#[tauri::command]
async fn generate_config_from_subscriptions(subscription_ids: Vec<String>) -> Result<String, String> {
    subscription::generate_config_from_subscriptions(subscription_ids).await
        .map_err(|e| format!("Failed to generate config: {}", e))
        .map(|_| "Configuration generated successfully".to_string())
}

#[tauri::command]
async fn enable_tun_mode(enable: bool) -> Result<String, String> {
    config::set_tun_mode(enable).await
        .map_err(|e| format!("Failed to set TUN mode: {}", e))
        .map(|_| if enable { "TUN mode enabled" } else { "TUN mode disabled" }.to_string())
}

#[tauri::command]
async fn check_mihomo_binary() -> Result<String, String> {
    let mihomo_paths = vec![
        "mihomo.exe",
        "mihomo", 
        "clash-meta.exe",
        "clash-meta",
        "./mihomo.exe",
        "./mihomo"
    ];
    
    for path in mihomo_paths {
        if let Ok(output) = std::process::Command::new(path)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                return Ok(format!("找到 mihomo: {} - {}", path, version.trim()));
            }
        }
    }
    
    Err("未找到 mihomo 二进制文件。请从 https://github.com/MetaCubeX/mihomo/releases 下载并安装。".to_string())
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
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("获取当前程序路径失败: {}", e))?;
        
        // 使用 PowerShell 以管理员身份重启应用
        let result = Command::new("powershell")
            .args([
                "-Command", 
                &format!("Start-Process '{}' -Verb RunAs", current_exe.display())
            ])
            .spawn();
        
        match result {
            Ok(_) => {
                // 启动成功，退出当前进程
                std::process::exit(0);
            }
            Err(e) => Err(format!("以管理员身份重启失败: {}", e))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Err("此功能仅在 Windows 系统上可用".to_string())
    }
}

#[tauri::command]
async fn get_bundled_mihomo_path() -> Result<String, String> {
    // 获取应用程序目录
    let app_dir = std::env::current_exe()
        .map_err(|e| format!("获取应用目录失败: {}", e))?
        .parent()
        .ok_or("无法获取应用目录")?
        .to_path_buf();
    
    // 简单方案：直接使用应用目录下的 mihomo.exe
    let mihomo_path = app_dir.join("mihomo.exe");
    
    if mihomo_path.exists() {
        Ok(mihomo_path.to_string_lossy().to_string())
    } else {
        Err(format!("未找到 mihomo.exe 文件。期望位置: {}", mihomo_path.display()))
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
        let config_dir = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo");
        
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
        
        let config_path = config_dir.join("config.yaml");
        
        // 如果配置文件不存在，创建默认配置
        if !config_path.exists() {
            crate::config::save_config(crate::mihomo::create_default_config()).await
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
            Err(format!("服务安装失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Err("服务安装功能仅在 Windows 系统上可用".to_string())
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
            Err(format!("服务启动失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Err("服务控制功能仅在 Windows 系统上可用".to_string())
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
            Err(format!("服务停止失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Err("服务控制功能仅在 Windows 系统上可用".to_string())
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
            Err(format!("服务卸载失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        Err("服务卸载功能仅在 Windows 系统上可用".to_string())
    }
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
        Ok("not_supported".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppStateType::new(AppState::default()))
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
            check_mihomo_binary,
            check_admin_privileges,
            restart_as_admin,
            get_bundled_mihomo_path,
            install_mihomo_service,
            start_mihomo_service_cmd,
            stop_mihomo_service_cmd,
            uninstall_mihomo_service,
            get_mihomo_service_status
        ])
        .setup(|app| {
            // Initialize application
            let window = app.get_window("main").unwrap();
            window.set_title("Mihomo Manager")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
