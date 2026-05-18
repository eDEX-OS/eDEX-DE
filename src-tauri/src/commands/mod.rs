pub mod greet;
pub mod settings;
pub mod terminal;
pub mod filesystem;
pub mod sysinfo;
pub mod audio;
pub mod audio_control;
pub mod launcher;
pub mod hyprland;
pub mod network;
pub mod fingerprint;
pub mod systemd;
pub mod update_checker;

#[allow(unused_imports)]
pub use greet::greet;
#[allow(unused_imports)]
pub use settings::{get_config_dir, load_settings, load_shortcuts, save_settings, save_shortcuts};
#[allow(unused_imports)]
pub use terminal::{close_terminal, list_terminals, spawn_terminal};
#[allow(unused_imports)]
pub use filesystem::{create_directory, delete_entry, fuzzy_search_files, list_dir, read_file, rename_entry};
#[allow(unused_imports)]
pub use sysinfo::{get_battery_info, get_cpu_info, get_disk_info, get_net_stats, get_process_list, get_ram_info, get_system_overview};
#[allow(unused_imports)]
pub use audio::play_audio;
#[allow(unused_imports)]
pub use audio_control::{audio_available, get_master_volume, list_audio_sinks, set_default_sink, set_master_volume, toggle_mute};
#[allow(unused_imports)]
pub use launcher::{get_hyprland_launcher_bind, launch_app, list_apps, search_apps};
#[allow(unused_imports)]
pub use hyprland::{
    generate_hyprland_config, get_active_window, get_hyprland_status, get_monitors,
    get_workspaces, hypr_dispatch, save_hyprland_integration_config, switch_workspace,
};
#[allow(unused_imports)]
pub use network::{get_active_connection_info, list_connections, network_available, nm_disconnect, wifi_connect, wifi_scan};
#[allow(unused_imports)]
pub use fingerprint::{fprintd_status, fprintd_verify};
#[allow(unused_imports)]
pub use systemd::{get_unit_logs, get_unit_status, list_units, unit_action};
#[allow(unused_imports)]
pub use update_checker::check_for_update;

pub mod bluetooth;
pub mod display;
pub mod power;
pub mod users;
pub mod notifications;
#[allow(unused_imports)]
pub use bluetooth::{bluetooth_available, bluetooth_list_devices, bluetooth_scan, bluetooth_connect, bluetooth_disconnect, bluetooth_pair, bluetooth_remove};
#[allow(unused_imports)]
pub use display::{get_display_info, set_monitor_config, set_keyboard_layout, set_mouse_sensitivity, set_natural_scroll};
#[allow(unused_imports)]
pub use power::{get_power_settings, set_power_settings, get_battery_status};
#[allow(unused_imports)]
pub use users::{list_users, change_password};
#[allow(unused_imports)]
pub use notifications::{get_notification_config, set_notification_config};

pub mod privacy;
#[allow(unused_imports)]
pub use privacy::{
    tailscale_available, tailscale_status, tailscale_login, tailscale_logout,
    tailscale_up, tailscale_down, tailscale_set_exit_node,
    tor_available, tor_status, tor_get_mode, tor_set_mode,
    tor_request_bridges, tor_get_bridges, tor_set_bridges,
    vpn_list_connections, vpn_connect, vpn_disconnect, vpn_import_wireguard,
};
