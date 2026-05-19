//! eDEX-DE settings system.
//!
//! TOML-based persistent configuration (~/.config/edex-de/config.toml).
//! Provides settings structs used by all other crates.

pub mod config;

pub use config::EdexConfig;

/// Returns the eDEX-DE config directory: `~/.config/edex-de/`.
pub fn config_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home).join(".config/edex-de")
}

/// Returns the path to the main config file: `~/.config/edex-de/config.toml`.
pub fn config_path() -> std::path::PathBuf {
    config_dir().join("config.toml")
}
