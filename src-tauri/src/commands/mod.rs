pub mod greet;
pub mod settings;
pub mod terminal;
pub mod filesystem;
pub mod sysinfo;
pub mod audio;
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
pub use update_checker::check_for_update;
