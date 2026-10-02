//! `/etc/edex-greeter/greeter.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GreeterConfig {
    /// Theme name from /usr/share/edex-de/themes.
    pub theme: String,
    /// Optional PNG background; the hex grid is drawn when absent.
    pub background: Option<PathBuf>,
    /// Session file name (without .desktop) selected by default.
    pub default_session: String,
    /// Command run for the default session when no session files exist.
    pub fallback_command: String,
    pub show_users: bool,
    pub power_buttons: bool,
    pub font_size: f32,
    /// Scan line and border pulse (turn off on very slow software rendering).
    pub animations: bool,
    /// Users below this uid are hidden.
    pub min_uid: u32,
    /// State file remembering the last user/session.
    pub state_file: PathBuf,
}

impl Default for GreeterConfig {
    fn default() -> Self {
        Self {
            theme: "tron".into(),
            background: None,
            default_session: "edex-de".into(),
            fallback_command: "edex-session".into(),
            show_users: true,
            power_buttons: true,
            font_size: 15.0,
            animations: true,
            min_uid: 1000,
            state_file: PathBuf::from("/var/cache/edex-greeter/state.toml"),
        }
    }
}

pub const DEFAULT_PATH: &str = "/etc/edex-greeter/greeter.toml";

pub fn load(path: &Path) -> GreeterConfig {
    match std::fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<GreeterConfig>(&text) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "invalid greeter config {}: {e}; using defaults",
                    path.display()
                );
                GreeterConfig::default()
            }
        },
        Err(_) => GreeterConfig::default(),
    }
}

/// Remembered last login.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct State {
    pub last_user: String,
    pub last_session: String,
}

pub fn load_state(path: &Path) -> State {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_state(path: &Path, state: &State) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = toml::to_string(state) {
        if let Err(e) = std::fs::write(path, text) {
            tracing::warn!("cannot save greeter state {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_config_merges_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("greeter.toml");
        std::fs::write(&p, "theme = \"matrix\"\nshow_users = false\n").unwrap();
        let c = load(&p);
        assert_eq!(c.theme, "matrix");
        assert!(!c.show_users);
        assert_eq!(c.default_session, "edex-de");
    }

    #[test]
    fn state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("nested/state.toml");
        save_state(
            &p,
            &State {
                last_user: "ari".into(),
                last_session: "edex-de".into(),
            },
        );
        assert_eq!(load_state(&p).last_user, "ari");
    }
}
