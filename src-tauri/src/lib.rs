use crate::state::AppState;
use tauri::Emitter;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod commands;
mod hyprland;
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
            let alt_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.global_shortcut().register(alt_space)?;

            if crate::hyprland::ipc::is_hyprland_running() {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    crate::hyprland::ipc::start_event_listener(handle).await;
                });
            }

            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running eDEX-DE");
}
