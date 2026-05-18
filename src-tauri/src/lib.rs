use crate::state::AppState;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Returns true when the app is running as the eDEX-DE Wayland session
/// (launched by edex-de-session via the display manager).
fn is_session_mode() -> bool {
    matches!(
        std::env::var("XDG_SESSION_DESKTOP").as_deref(),
        Ok("eDEX-DE") | Ok("edex-de")
    )
}

mod commands;
mod hyprland;
mod layer_shell;
mod pty;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "edex_de=info".into()),
        )
        .init();

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let alt_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
                        if shortcut == &alt_space {
                            let _ = app.emit("toggle-launcher", ());
                        }
                    }
                })
                .build(),
        )
        .manage(app_state)
        .setup(|app| {
            let launch_args: Vec<String> = std::env::args().skip(1).collect();
            if !launch_args.is_empty() {
                tracing::info!(?launch_args, "CLI args detected; external IPC dispatch is not implemented yet");
            }

            let alt_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.global_shortcut().register(alt_space)?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_fullscreen(true);
                let _ = window.set_decorations(false);
                let _ = window.set_resizable(false);
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: 0, y: 0 }));
                crate::layer_shell::try_apply_layer_shell(&window);
            }

            if crate::hyprland::ipc::is_hyprland_running() {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    crate::hyprland::ipc::start_event_listener(handle).await;
                });
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // When running as the DE session, ignore close requests so the
            // eDEX shell can never be accidentally dismissed via Alt+F4 or
            // window manager close buttons.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && is_session_mode() {
                    api.prevent_close();
                    tracing::debug!("CloseRequested ignored — running as eDEX-DE session");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet::greet,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::load_shortcuts,
            commands::settings::save_shortcuts,
            commands::settings::get_config_dir,
            commands::terminal::spawn_terminal,
            commands::terminal::close_terminal,
            commands::terminal::list_terminals,
            commands::filesystem::list_dir,
            commands::filesystem::read_file,
            commands::filesystem::rename_entry,
            commands::filesystem::delete_entry,
            commands::filesystem::create_directory,
            commands::filesystem::fuzzy_search_files,
            commands::sysinfo::get_cpu_info,
            commands::sysinfo::get_ram_info,
            commands::sysinfo::get_net_stats,
            commands::sysinfo::get_process_list,
            commands::sysinfo::get_system_overview,
            commands::sysinfo::get_disk_info,
            commands::sysinfo::get_battery_info,
            commands::audio::play_audio,
            commands::audio_control::audio_available,
            commands::audio_control::list_audio_sinks,
            commands::audio_control::get_master_volume,
            commands::audio_control::set_master_volume,
            commands::audio_control::toggle_mute,
            commands::audio_control::set_default_sink,
            commands::update_checker::check_for_update,
            commands::network::network_available,
            commands::network::list_connections,
            commands::network::wifi_scan,
            commands::network::wifi_connect,
            commands::network::nm_disconnect,
            commands::network::get_active_connection_info,
            commands::fingerprint::fprintd_status,
            commands::fingerprint::fprintd_verify,
            commands::systemd::list_units,
            commands::systemd::unit_action,
            commands::systemd::get_unit_logs,
            commands::systemd::get_unit_status,
            commands::launcher::list_apps,
            commands::launcher::search_apps,
            commands::launcher::launch_app,
            commands::launcher::get_hyprland_launcher_bind,
            commands::hyprland::get_hyprland_status,
            commands::hyprland::get_workspaces,
            commands::hyprland::get_active_window,
            commands::hyprland::get_monitors,
            commands::hyprland::switch_workspace,
            commands::hyprland::hypr_dispatch,
            commands::hyprland::generate_hyprland_config,
            commands::hyprland::save_hyprland_integration_config,
            commands::privacy::tailscale_available,
            commands::privacy::tailscale_status,
            commands::privacy::tailscale_login,
            commands::privacy::tailscale_logout,
            commands::privacy::tailscale_up,
            commands::privacy::tailscale_down,
            commands::privacy::tailscale_set_exit_node,
            commands::privacy::tor_available,
            commands::privacy::tor_status,
            commands::privacy::tor_get_mode,
            commands::privacy::tor_set_mode,
            commands::privacy::tor_request_bridges,
            commands::privacy::tor_get_bridges,
            commands::privacy::tor_set_bridges,
            commands::privacy::vpn_list_connections,
            commands::privacy::vpn_connect,
            commands::privacy::vpn_disconnect,
            commands::privacy::vpn_import_wireguard,
            // Bluetooth
            commands::bluetooth::bluetooth_available,
            commands::bluetooth::bluetooth_list_devices,
            commands::bluetooth::bluetooth_scan,
            commands::bluetooth::bluetooth_connect,
            commands::bluetooth::bluetooth_disconnect,
            commands::bluetooth::bluetooth_pair,
            commands::bluetooth::bluetooth_remove,
            // Display / Input
            commands::display::get_display_info,
            commands::display::set_monitor_config,
            commands::display::set_keyboard_layout,
            commands::display::set_mouse_sensitivity,
            commands::display::set_natural_scroll,
            // Power
            commands::power::get_power_settings,
            commands::power::set_power_settings,
            commands::power::get_battery_status,
            // Users
            commands::users::list_users,
            commands::users::change_password,
            // Notifications
            commands::notifications::get_notification_config,
            commands::notifications::set_notification_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running eDEX-DE");
}
