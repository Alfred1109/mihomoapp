use std::sync::Arc;

use tauri::State;

use crate::watchdog;

#[cfg(target_os = "windows")]
pub(crate) mod windows_service {
    use std::path::PathBuf;
    use std::process::Command;

    pub struct ServicePaths {
        pub winsw_exe: PathBuf,
    }

    pub fn get_service_paths() -> Result<ServicePaths, String> {
        let app_dir = std::env::current_exe()
            .map_err(|e| format!("获取应用目录失败: {}", e))?
            .parent()
            .ok_or("无法获取应用目录")?
            .to_path_buf();

        let winsw_source = {
            let direct = app_dir.join("winsw.exe");
            if direct.exists() {
                direct
            } else {
                let resource = app_dir.join("resources").join("winsw.exe");
                if resource.exists() {
                    resource
                } else {
                    return Err(format!(
                        "未找到 winsw.exe 文件。期望位置: {}",
                        direct.display()
                    ));
                }
            }
        };

        let mihomo_path = {
            let mihomo = app_dir.join("mihomo.exe");
            if mihomo.exists() {
                mihomo
            } else {
                let resource = app_dir.join("resources").join("mihomo.exe");
                if resource.exists() {
                    std::fs::copy(&resource, &mihomo)
                        .map_err(|e| format!("复制 mihomo.exe 失败: {}", e))?;
                    mihomo
                } else {
                    return Err(format!(
                        "未找到 mihomo.exe 文件。期望位置: {}",
                        mihomo.display()
                    ));
                }
            }
        };

        let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?.join("mihomo");

        let winsw_exe = app_dir.join("MihomoService.exe");
        let winsw_xml = app_dir.join("MihomoService.xml");

        if !winsw_exe.exists() {
            std::fs::copy(&winsw_source, &winsw_exe)
                .map_err(|e| format!("复制 WinSW 失败: {}", e))?;
        }

        let xml_content = format!(
            r#"<service>
  <id>MihomoService</id>
  <name>Mihomo Proxy Service</name>
  <description>Mihomo Proxy Service</description>
  <executable>{}</executable>
  <arguments>-d "{}"</arguments>
  <logpath>{}</logpath>
  <log mode="roll" />
  <startmode>Automatic</startmode>
  <onfailure action="restart" delay="5 sec"/>
</service>"#,
            mihomo_path.display(),
            config_dir.display(),
            app_dir.display()
        );

        std::fs::write(&winsw_xml, xml_content)
            .map_err(|e| format!("写入 WinSW 配置失败: {}", e))?;

        Ok(ServicePaths { winsw_exe })
    }

    pub fn is_service_installed() -> bool {
        Command::new("sc")
            .args(["query", "MihomoService"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn kill_mihomo_process() {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "mihomo.exe"])
            .output();
    }
}

pub(crate) async fn wait_for_service_start() -> bool {
    for _ in 0..10 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if crate::mihomo::is_mihomo_running().await {
            return true;
        }
    }
    false
}

#[tauri::command]
pub async fn install_mihomo_service(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    watchdog.clear_process().await;

    let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?.join("mihomo");
    std::fs::create_dir_all(&config_dir).map_err(|e| format!("创建配置目录失败: {}", e))?;

    let config_path = config_dir.join("config.yaml");
    if !config_path.exists() {
        crate::config::save_config(crate::mihomo::create_default_config())
            .await
            .map_err(|e| format!("创建默认配置失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let paths = windows_service::get_service_paths()?;

        if windows_service::is_service_installed() {
            let _ = Command::new(&paths.winsw_exe)
                .args(["stop", "--force"])
                .output();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            windows_service::kill_mihomo_process();
            let _ = Command::new(&paths.winsw_exe).arg("uninstall").output();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        } else {
            windows_service::kill_mihomo_process();
        }

        let output = Command::new(&paths.winsw_exe)
            .arg("install")
            .output()
            .map_err(|e| format!("安装服务失败: {}", e))?;

        if output.status.success() {
            Ok("mihomo 服务安装成功，请点击「启动」按钮启动服务".to_string())
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

        let mihomo_binary = ["/usr/local/bin/mihomo", "/usr/bin/mihomo"]
            .iter()
            .find(|p| Path::new(p).exists())
            .map(|s| s.to_string())
            .or_else(|| {
                Command::new("which")
                    .arg("mihomo")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            })
            .ok_or("未找到 mihomo 二进制文件，请先安装 mihomo")?;

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
            config_path.parent().ok_or("无法获取配置目录")?.display()
        );

        fs::write("/etc/systemd/system/mihomo.service", service_content)
            .map_err(|e| format!("写入服务文件失败 (需要 root 权限): {}", e))?;

        for (cmd, args, err_msg) in [
            ("systemctl", vec!["daemon-reload"], "重新加载 systemd 失败"),
            (
                "systemctl",
                vec!["enable", "mihomo.service"],
                "启用服务失败",
            ),
            ("systemctl", vec!["start", "mihomo.service"], "启动服务失败"),
        ] {
            let output = Command::new(cmd)
                .args(&args)
                .output()
                .map_err(|e| format!("{}: {}", err_msg, e))?;
            if !output.status.success() {
                return Err(format!(
                    "{}: {}",
                    err_msg,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        wait_for_service_start().await;
        watchdog.set_process(0).await;

        Ok("Mihomo 服务安装、启用并启动成功".to_string())
    }
}

#[tauri::command]
pub async fn start_mihomo_service_cmd(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        if !windows_service::is_service_installed() {
            return Err("MihomoService 服务不存在，请先安装服务".to_string());
        }

        let paths = windows_service::get_service_paths()?;
        let output = Command::new(&paths.winsw_exe)
            .arg("start")
            .output()
            .map_err(|e| format!("启动服务命令执行失败: {}", e))?;

        if output.status.success() {
            let started = wait_for_service_start().await;
            watchdog.set_process(0).await;

            if started {
                Ok("mihomo 服务启动成功".to_string())
            } else {
                Ok("mihomo 服务启动命令已执行，等待服务完全启动...".to_string())
            }
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

        let check_output = Command::new("systemctl")
            .args(["list-unit-files", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务失败: {}", e))?;

        if !String::from_utf8_lossy(&check_output.stdout).contains("mihomo.service") {
            return Err("Mihomo 服务未安装，请先安装服务".to_string());
        }

        let output = Command::new("pkexec")
            .args(["systemctl", "start", "mihomo.service"])
            .output()
            .map_err(|e| format!("启动服务失败: {}", e))?;

        if output.status.success() {
            let started = wait_for_service_start().await;
            watchdog.set_process(0).await;

            if started {
                Ok("Mihomo 服务启动成功".to_string())
            } else {
                Ok("Mihomo 服务启动命令已执行，等待服务完全启动...".to_string())
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("服务启动失败: {}", stderr))
        }
    }
}

#[tauri::command]
pub async fn stop_mihomo_service_cmd(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    watchdog.clear_process().await;

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let paths = windows_service::get_service_paths()?;

        let output = Command::new(&paths.winsw_exe)
            .arg("stop")
            .output()
            .map_err(|e| format!("停止服务失败: {}", e))?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        windows_service::kill_mihomo_process();

        if output.status.success() {
            Ok("mihomo 服务停止成功".to_string())
        } else {
            Ok("mihomo 服务已停止".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;

        let output = Command::new("pkexec")
            .args(["systemctl", "stop", "mihomo.service"])
            .output()
            .map_err(|e| format!("停止服务失败: {}", e))?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let _ = Command::new("pkill").arg("mihomo").output();

        if output.status.success() {
            Ok("Mihomo 服务停止成功".to_string())
        } else {
            Ok("Mihomo 服务已停止".to_string())
        }
    }
}

#[tauri::command]
pub async fn restart_mihomo_service_cmd(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let paths = windows_service::get_service_paths()?;

        let output = Command::new(&paths.winsw_exe)
            .arg("restart")
            .output()
            .map_err(|e| format!("重启服务失败: {}", e))?;

        if output.status.success() {
            wait_for_service_start().await;
            watchdog.set_process(0).await;
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

        let output = Command::new("pkexec")
            .args(["systemctl", "restart", "mihomo.service"])
            .output()
            .map_err(|e| format!("重启服务失败: {}", e))?;

        if output.status.success() {
            wait_for_service_start().await;
            watchdog.set_process(0).await;
            Ok("Mihomo 服务重启成功".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("服务重启失败: {}", stderr))
        }
    }
}

#[tauri::command]
pub async fn uninstall_mihomo_service(
    watchdog: State<'_, Arc<watchdog::ProcessWatchdog>>,
) -> Result<String, String> {
    watchdog.clear_process().await;

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let paths = windows_service::get_service_paths()?;

        let _ = Command::new(&paths.winsw_exe).arg("stop").output();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        windows_service::kill_mihomo_process();

        let output = Command::new(&paths.winsw_exe)
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

        let _ = Command::new("systemctl")
            .args(["stop", "mihomo.service"])
            .output();
        let _ = Command::new("systemctl")
            .args(["disable", "mihomo.service"])
            .output();
        let _ = Command::new("pkill").arg("mihomo").output();

        match fs::remove_file("/etc/systemd/system/mihomo.service") {
            Ok(_) => {
                let _ = Command::new("systemctl").arg("daemon-reload").output();
                Ok("Mihomo 服务卸载成功".to_string())
            }
            Err(e) => Err(format!("删除服务文件失败 (需要 root 权限): {}", e)),
        }
    }
}

#[tauri::command]
pub async fn set_autostart(enable: bool) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let current_exe =
            std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

        let app_name = "MihomoManager";
        let reg_key = r"Software\Microsoft\Windows\CurrentVersion\Run";

        if enable {
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
            fs::create_dir_all(&autostart_dir)
                .map_err(|e| format!("创建 autostart 目录失败: {}", e))?;

            let current_exe =
                std::env::current_exe().map_err(|e| format!("获取当前程序路径失败: {}", e))?;

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
pub async fn get_autostart_status() -> Result<bool, String> {
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
pub async fn set_silent_start(enable: bool) -> Result<String, String> {
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
pub async fn get_silent_start_status() -> Result<bool, String> {
    let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?;
    let settings_file = config_dir.join("mihomo-manager/settings.json");

    if !settings_file.exists() {
        return Ok(false);
    }

    let content =
        std::fs::read_to_string(&settings_file).map_err(|e| format!("读取设置文件失败: {}", e))?;

    let settings: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析设置文件失败: {}", e))?;

    Ok(settings["silent_start"].as_bool().unwrap_or(false))
}

#[tauri::command]
pub async fn get_mihomo_service_status() -> Result<String, String> {
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

        let unit_files_output = Command::new("systemctl")
            .args(["list-unit-files", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务安装状态失败: {}", e))?;

        if !String::from_utf8_lossy(&unit_files_output.stdout).contains("mihomo.service") {
            return Ok("not_installed".to_string());
        }

        let status_output = Command::new("systemctl")
            .args(["is-active", "mihomo.service"])
            .output()
            .map_err(|e| format!("检查服务状态失败: {}", e))?;

        let status_str = String::from_utf8_lossy(&status_output.stdout);
        let status = status_str.trim();

        match status {
            "active" | "activating" | "reloading" => Ok("running".to_string()),
            "inactive" | "deactivating" | "failed" => Ok("stopped".to_string()),
            _ => Ok("installed".to_string()),
        }
    }
}
