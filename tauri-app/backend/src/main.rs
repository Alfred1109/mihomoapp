// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod base_config;
mod commands;
mod config;
mod config_manager;
mod error;
mod events;
mod mihomo;
mod platform_config;
mod subscription;
mod validator;
mod watchdog;

use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};

use commands::service as service_commands;
use commands::{
    add_subscription, backup_subscriptions, check_admin_privileges, check_mihomo_binary,
    delete_config_backup, delete_subscription, enable_tun_mode, export_base_config,
    export_subscriptions, generate_config_from_subscriptions, get_auto_restart,
    get_autostart_status, get_base_config, get_bundled_mihomo_path, get_current_ip,
    get_mihomo_config, get_mihomo_service_status, get_mihomo_status, get_proxies,
    get_silent_start_status, get_subscriptions, import_base_config, import_subscriptions,
    install_mihomo_service, list_config_backups, list_subscription_backups,
    regenerate_runtime_config, rename_config_backup, reset_config_to_default, restart_as_admin,
    restart_mihomo_service_cmd, restore_config_backup, restore_subscriptions_from_backup,
    save_base_config, save_mihomo_config, set_auto_restart, set_autostart, set_silent_start,
    start_mihomo_service_cmd, stop_mihomo_service_cmd, switch_proxy, test_all_proxies,
    test_group_delay, uninstall_mihomo_service, update_subscription, validate_config,
};

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.hide();
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
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => show_main_window(app),
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "show" => show_main_window(app),
                    "hide" => hide_main_window(app),
                    "start" => {
                        // 启动服务
                        let watchdog = app
                            .state::<std::sync::Arc<watchdog::ProcessWatchdog>>()
                            .inner()
                            .clone();
                        tauri::async_runtime::spawn(async move {
                            #[cfg(target_os = "windows")]
                            {
                                use std::process::Command;
                                // 优先使用系统服务
                                if service_commands::windows_service::is_service_installed() {
                                    if let Ok(paths) =
                                        service_commands::windows_service::get_service_paths()
                                    {
                                        let _ =
                                            Command::new(&paths.winsw_exe).arg("start").output();
                                        service_commands::wait_for_service_start().await;
                                        watchdog.set_process(0).await;
                                        return;
                                    }
                                }
                            }

                            #[cfg(not(target_os = "windows"))]
                            {
                                use std::process::Command;
                                // 尝试通过 systemd 启动
                                if Command::new("systemctl")
                                    .args(["start", "mihomo.service"])
                                    .output()
                                    .is_ok()
                                {
                                    service_commands::wait_for_service_start().await;
                                    watchdog.set_process(0).await;
                                    return;
                                }
                            }

                            // 回退到直接进程方式
                            if let Ok(pid) = mihomo::start_mihomo().await {
                                watchdog.set_process(pid).await;
                            }
                        });
                    }
                    "stop" => {
                        // 停止服务
                        let watchdog = app
                            .state::<std::sync::Arc<watchdog::ProcessWatchdog>>()
                            .inner()
                            .clone();
                        tauri::async_runtime::spawn(async move {
                            watchdog.clear_process().await;

                            #[cfg(target_os = "windows")]
                            {
                                use std::process::Command;
                                if let Ok(paths) =
                                    service_commands::windows_service::get_service_paths()
                                {
                                    let _ = Command::new(&paths.winsw_exe).arg("stop").output();
                                }
                                service_commands::windows_service::kill_mihomo_process();
                            }

                            #[cfg(not(target_os = "windows"))]
                            {
                                use std::process::Command;
                                let _ = Command::new("systemctl")
                                    .args(["stop", "mihomo.service"])
                                    .output();
                                let _ = Command::new("pkill").arg("mihomo").output();
                            }
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
            get_silent_start_status,
            reset_config_to_default,
            export_subscriptions,
            import_subscriptions,
            backup_subscriptions,
            list_subscription_backups,
            restore_subscriptions_from_backup,
            export_base_config,
            import_base_config,
            get_base_config,
            save_base_config,
            regenerate_runtime_config
        ])
        .setup(|app| {
            // Initialize application
            let window = app.get_window("main").ok_or("Failed to get main window")?;
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
