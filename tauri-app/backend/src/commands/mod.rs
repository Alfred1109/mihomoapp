pub mod config_subscription;
pub mod runtime_system;
pub mod service;

pub use config_subscription::{
    add_subscription, delete_subscription, enable_tun_mode, export_base_config,
    export_subscriptions, generate_config_from_subscriptions, get_base_config, get_mihomo_config,
    get_subscriptions, import_base_config, import_subscriptions, list_subscription_backups,
    regenerate_runtime_config, reset_config_to_default, restore_subscriptions_from_backup,
    save_base_config, save_mihomo_config, update_subscription,
};
pub use runtime_system::{
    backup_subscriptions, check_admin_privileges, check_mihomo_binary, delete_config_backup,
    get_auto_restart, get_bundled_mihomo_path, get_current_ip, get_mihomo_status, get_proxies,
    list_config_backups, rename_config_backup, restart_as_admin, restore_config_backup,
    set_auto_restart, switch_proxy, test_all_proxies, test_group_delay, validate_config,
};
pub use service::{
    get_autostart_status, get_mihomo_service_status, get_silent_start_status,
    install_mihomo_service, restart_mihomo_service_cmd, set_autostart, set_silent_start,
    start_mihomo_service_cmd, stop_mihomo_service_cmd, uninstall_mihomo_service,
};
