use crate::state::AppState;

mod commands;
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
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
            commands::update_checker::check_for_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running eDEX-DE");
}
