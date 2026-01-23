// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mihomo;
mod config;
mod subscription;
mod backup;
mod validator;

use std::sync::Mutex;
use tauri::{Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem, SystemTrayMenuItem};
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
async fn add_subscription(name: String, url: String, user_agent: Option<String>, use_proxy: bool) -> Result<String, String> {
    subscription::add_subscription(name, url, user_agent, use_proxy).await
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
async fn test_group_delay(group_name: String) -> Result<String, String> {
    mihomo::test_group_delay(&group_name).await
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
        "http://ip-api.com/json/",  // 免费，返回完整地理位置信息
        "https://ipapi.co/json/",
        "https://api.ip.sb/geoip",
    ];
    
    for service in services {
        println!("尝试从 {} 获取IP信息", service);
        if let Ok(response) = client.get(service).send().await {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                println!("成功获取IP信息: {:?}", data);
                return Ok(data);
            }
        }
    }
    
    Err("Failed to get IP information from all services".to_string())
}

#[tauri::command]
async fn test_all_proxies(test_url: Option<String>, timeout: Option<u32>) -> Result<serde_json::Value, String> {
    mihomo::test_all_proxies_delay(test_url, timeout).await
        .map_err(|e| format!("Failed to test all proxies: {}", e))
}

#[tauri::command]
async fn validate_config(config: serde_json::Value) -> Result<validator::ValidationResult, String> {
    validator::validate_config(&config).await
        .map_err(|e| format!("Failed to validate config: {}", e))
}

#[tauri::command]
async fn list_config_backups() -> Result<Vec<String>, String> {
    backup::list_backups().await
        .map_err(|e| format!("Failed to list backups: {}", e))
}

#[tauri::command]
async fn restore_config_backup(backup_filename: String) -> Result<String, String> {
    backup::restore_config(&backup_filename).await
        .map_err(|e| format!("Failed to restore backup: {}", e))
        .map(|_| "配置已从备份恢复，请重启服务以应用更改".to_string())
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
        use std::process::Command;
        
        // 获取当前可执行文件路径
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("获取当前程序路径失败: {}", e))?;
        
        // 在 Linux 上，GUI程序以root权限重启比较复杂
        // 需要保持环境变量和桌面会话访问权限
        let display = std::env::var("DISPLAY").unwrap_or(":0".to_string());
        let xauth = std::env::var("XAUTHORITY").ok();
        
        // 构建命令，保持必要的环境变量
        let mut restart_cmd = vec![];
        restart_cmd.push("env".to_string());
        restart_cmd.push(format!("DISPLAY={}", display));
        if let Some(xauth_path) = xauth {
            restart_cmd.push(format!("XAUTHORITY={}", xauth_path));
        }
        restart_cmd.push(current_exe.to_string_lossy().to_string());
        
        // 尝试不同的权限提升方式
        let commands_to_try = vec![
            ("pkexec", restart_cmd.clone()),
            ("sudo", restart_cmd.clone()),
        ];
        
        for (cmd, args) in commands_to_try {
            match Command::new(cmd).args(&args).spawn() {
                Ok(mut child) => {
                    // 等待一小段时间确保新进程启动
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    
                    // 检查子进程是否还在运行
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            // 子进程已经退出，可能启动失败
                            continue;
                        }
                        Ok(None) => {
                            // 子进程还在运行，认为启动成功
                            std::process::exit(0);
                        }
                        Err(_) => continue,
                    }
                }
                Err(_) => continue,
            }
        }
        
        Err("无法以root权限重启应用。请尝试在终端中手动使用 'sudo mihomo-manager' 运行".to_string())
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
        use std::process::Command;
        use std::fs;
        use std::path::Path;

        // 创建配置目录
        let config_dir = dirs::config_dir()
            .ok_or("无法获取配置目录")?
            .join("mihomo");
        
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
        
        let config_path = config_dir.join("config.yaml");
        
        // 如果配置文件不存在，创建默认配置
        if !config_path.exists() {
            crate::config::save_config(crate::mihomo::create_default_config()).await
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
                _ => "/usr/local/bin/mihomo".to_string() // 默认路径
            }
        };

        // 创建 systemd 服务文件内容
        let service_content = format!(r#"[Unit]
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
"#, mihomo_binary, config_path.parent().unwrap().display());

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
            Err(format!("服务启动失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
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
        
        // 启动服务
        let output = Command::new("systemctl")
            .args(["start", "mihomo.service"])
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
            Err(format!("服务停止失败:\n标准输出: {}\n错误输出: {}", stdout, stderr))
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        
        // 停止服务
        let output = Command::new("systemctl")
            .args(["stop", "mihomo.service"])
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
        use std::process::Command;
        use std::fs;
        
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
                let _ = Command::new("systemctl")
                    .arg("daemon-reload")
                    .output();
                
                Ok("Mihomo 服务卸载成功".to_string())
            }
            Err(e) => Err(format!("删除服务文件失败 (需要 root 权限): {}", e))
        }
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
            "inactive" => Ok("stopped".to_string()),
            "failed" => Ok("stopped".to_string()),
            _ => Ok("installed".to_string())
        }
    }
}

fn main() {
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
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "show" => {
                        let window = app.get_window("main").unwrap();
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                    "hide" => {
                        let window = app.get_window("main").unwrap();
                        window.hide().unwrap();
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
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // 关闭窗口时最小化到托盘而不是退出
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
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
            test_group_delay,
            test_all_proxies,
            validate_config,
            list_config_backups,
            restore_config_backup,
            get_current_ip,
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
