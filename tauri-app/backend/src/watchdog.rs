use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[path = "watchdog_state.rs"]
mod state;
use state::WatchdogState;

pub struct ProcessWatchdog {
    state: Arc<RwLock<WatchdogState>>,
    app_handle: tauri::AppHandle,
    monitoring: Arc<RwLock<bool>>,
}

async fn check_mihomo_api_health() -> bool {
    crate::mihomo::is_mihomo_running().await
}

impl ProcessWatchdog {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            state: Arc::new(RwLock::new(WatchdogState::default())),
            app_handle,
            monitoring: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn set_process(&self, pid: u32) {
        self.state.write().await.started(pid);
        info!("Watchdog tracking process: {}", pid);
    }

    pub async fn clear_process(&self) {
        self.take_managed_process().await;
        info!("Watchdog cleared process tracking (manual stop)");
    }

    /// Atomically relinquish ownership intent before the caller performs a stop.
    /// None is unmanaged, zero is a system service, and a positive PID is an owned child.
    pub async fn take_managed_process(&self) -> Option<u32> {
        self.state.write().await.stopped()
    }

    pub async fn set_auto_restart(&self, enabled: bool) {
        self.state.write().await.auto_restart = enabled;
        info!("Auto-restart set to: {}", enabled);
    }

    pub async fn get_auto_restart(&self) -> bool {
        self.state.read().await.auto_restart
    }

    pub async fn start_monitoring(&self) {
        let mut monitoring = self.monitoring.write().await;
        if *monitoring {
            warn!("Watchdog already monitoring");
            return;
        }
        *monitoring = true;
        drop(monitoring);

        let state = self.state.clone();
        let app_handle = self.app_handle.clone();
        let monitoring_flag = self.monitoring.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3));
            let mut last_known_status = None;
            info!("Watchdog monitoring started (API health check mode)");

            loop {
                interval.tick().await;
                if !*monitoring_flag.read().await {
                    info!("Watchdog monitoring stopped");
                    break;
                }

                let api_healthy = check_mihomo_api_health().await;
                let health_changed = last_known_status != Some(api_healthy);
                if health_changed {
                    last_known_status = Some(api_healthy);
                    crate::events::emit_mihomo_status(
                        &app_handle,
                        crate::events::MihomoStatusEvent {
                            running: api_healthy,
                            process_id: state.read().await.process_id,
                            timestamp: crate::events::get_current_timestamp(),
                        },
                    );
                }

                // Keep the intent lock until the start operation finishes. A stop
                // request then clears intent before its caller stops that process.
                let mut runtime = state.write().await;
                let process_exited = if runtime.process_id.is_some_and(|pid| pid > 0) {
                    match crate::mihomo::owned_process_exited().await {
                        Ok(exited) => exited,
                        Err(e) => {
                            error!("Cannot inspect owned Mihomo process: {}", e);
                            continue;
                        }
                    }
                } else {
                    false
                };
                if !api_healthy && !process_exited && health_changed {
                    warn!("Mihomo controller is unavailable; no owned process exit was observed");
                }
                if let Some(attempt) =
                    runtime.reserve_restart(api_healthy, process_exited, std::time::Instant::now())
                {
                    info!("Auto-restarting mihomo (attempt {}/5)", attempt);
                    match crate::mihomo::start_mihomo().await {
                        Ok(pid) => {
                            runtime.process_id = Some(pid);
                            // Only an observed healthy controller resets the budget.
                            info!("Mihomo recovery started with PID: {}", pid);
                        }
                        Err(e) => error!("Failed to restart mihomo: {}", e),
                    }
                }
            }
        });
    }
}
