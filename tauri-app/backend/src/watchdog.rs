use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use sysinfo::{System, Pid, ProcessRefreshKind};
use tracing::{info, warn, error};

pub struct ProcessWatchdog {
    process_id: Arc<RwLock<Option<u32>>>,
    auto_restart: Arc<RwLock<bool>>,
    app_handle: tauri::AppHandle,
    monitoring: Arc<RwLock<bool>>,
    last_known_status: Arc<RwLock<bool>>,
}

/// 检查 mihomo API 是否可访问（健康检查）
async fn check_mihomo_api_health() -> bool {
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => {
            match client.get("http://127.0.0.1:9090/version").send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

impl ProcessWatchdog {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            process_id: Arc::new(RwLock::new(None)),
            auto_restart: Arc::new(RwLock::new(true)),
            app_handle,
            monitoring: Arc::new(RwLock::new(false)),
            last_known_status: Arc::new(RwLock::new(false)),
        }
    }
    
    pub async fn set_process(&self, pid: u32) {
        let mut process_id = self.process_id.write().await;
        *process_id = Some(pid);
        info!("Watchdog tracking process: {}", pid);
    }
    
    pub async fn clear_process(&self) {
        let mut process_id = self.process_id.write().await;
        *process_id = None;
        info!("Watchdog cleared process tracking");
    }
    
    pub async fn set_auto_restart(&self, enabled: bool) {
        let mut auto_restart = self.auto_restart.write().await;
        *auto_restart = enabled;
        info!("Auto-restart set to: {}", enabled);
    }
    
    pub async fn get_auto_restart(&self) -> bool {
        let auto_restart = self.auto_restart.read().await;
        *auto_restart
    }
    
    pub async fn start_monitoring(&self) {
        let mut monitoring = self.monitoring.write().await;
        if *monitoring {
            warn!("Watchdog already monitoring");
            return;
        }
        *monitoring = true;
        drop(monitoring);
        
        let process_id = self.process_id.clone();
        let auto_restart = self.auto_restart.clone();
        let app_handle = self.app_handle.clone();
        let monitoring_flag = self.monitoring.clone();
        let last_known_status = self.last_known_status.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3));
            let mut restart_count = 0;
            let max_restart_attempts = 5;
            let restart_window = Duration::from_secs(60);
            let mut last_restart_time = std::time::Instant::now();
            
            info!("Watchdog monitoring started (API health check mode)");
            
            loop {
                interval.tick().await;
                
                let should_continue = {
                    let monitoring = monitoring_flag.read().await;
                    *monitoring
                };
                
                if !should_continue {
                    info!("Watchdog monitoring stopped");
                    break;
                }
                
                // 使用 API 健康检查替代进程检查
                let api_healthy = check_mihomo_api_health().await;
                
                let last_status = {
                    let status = last_known_status.read().await;
                    *status
                };
                
                // 只在状态变化时发送事件
                if api_healthy != last_status {
                    info!("Mihomo status changed: {} -> {}", last_status, api_healthy);
                    
                    {
                        let mut status = last_known_status.write().await;
                        *status = api_healthy;
                    }
                    
                    crate::events::emit_mihomo_status(
                        &app_handle,
                        crate::events::MihomoStatusEvent {
                            running: api_healthy,
                            process_id: None,
                            timestamp: crate::events::get_current_timestamp(),
                        },
                    );
                    
                    // 如果服务停止且启用了自动重启
                    if !api_healthy {
                        warn!("Mihomo API is not responding, service may have stopped");
                        
                        {
                            let mut pid_lock = process_id.write().await;
                            *pid_lock = None;
                        }
                        
                        let should_restart = {
                            let restart_lock = auto_restart.read().await;
                            *restart_lock
                        };
                        
                        if should_restart {
                            let now = std::time::Instant::now();
                            if now.duration_since(last_restart_time) > restart_window {
                                restart_count = 0;
                            }
                            
                            if restart_count >= max_restart_attempts {
                                error!(
                                    "Max restart attempts ({}) reached within {} seconds, giving up",
                                    max_restart_attempts,
                                    restart_window.as_secs()
                                );
                                continue;
                            }
                            
                            restart_count += 1;
                            last_restart_time = now;
                            
                            info!("Auto-restarting mihomo (attempt {}/{})", restart_count, max_restart_attempts);
                            
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            
                            match crate::mihomo::start_mihomo().await {
                                Ok(new_pid) => {
                                    info!("Mihomo restarted successfully with PID: {}", new_pid);
                                    
                                    {
                                        let mut pid_lock = process_id.write().await;
                                        *pid_lock = Some(new_pid);
                                    }
                                    
                                    restart_count = 0;
                                }
                                Err(e) => {
                                    error!("Failed to restart mihomo: {}", e);
                                }
                            }
                        }
                    } else {
                        // 服务恢复运行，重置重启计数
                        restart_count = 0;
                    }
                }
            }
        });
    }
}
