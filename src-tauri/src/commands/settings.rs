use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub shell: String,
    pub shell_args: String,
    pub cwd: String,
    pub keyboard: String,
    pub theme: String,
    pub term_font_size: u32,
    pub shell_opacity: f32,
    pub audio: bool,
    pub audio_volume: f32,
    pub disable_feedback_audio: bool,
    pub clock_hours: u32,
    pub ping_addr: String,
    pub port: u16,
    pub nointro: bool,
    pub nocursor: bool,
    pub force_fullscreen: bool,
    pub allow_windowed: bool,
    pub exclude_threads_from_toplist: bool,
    pub hide_dotfiles: bool,
    pub fs_list_view: bool,
    pub experimental_globe_features: bool,
    pub experimental_features: bool,
    #[serde(default)]
    pub monitor: Option<u32>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shell: if cfg!(windows) {
                "powershell.exe".into()
            } else {
                "bash".into()
            },
            shell_args: String::new(),
            cwd: dirs::home_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            keyboard: "en-US".into(),
            theme: "tron".into(),
            term_font_size: 15,
            shell_opacity: 1.0,
            audio: true,
            audio_volume: 1.0,
            disable_feedback_audio: false,
            clock_hours: 24,
            ping_addr: "1.1.1.1".into(),
            port: 3000,
            nointro: false,
            nocursor: false,
            force_fullscreen: true,
            allow_windowed: false,
            exclude_threads_from_toplist: true,
            hide_dotfiles: false,
            fs_list_view: false,
            experimental_globe_features: false,
            experimental_features: false,
            monitor: None,
            env: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    #[serde(rename = "type")]
    pub shortcut_type: String,
    pub trigger: String,
    pub action: String,
    pub enabled: bool,
    #[serde(default)]
    pub linebreak: bool,
}

fn config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| dirs::config_dir().unwrap_or_default().join("edex-ui"))
}

fn settings_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join("settings.json")
}

fn shortcuts_path(app: &AppHandle) -> PathBuf {
    config_dir(app).join("shortcuts.json")
}

fn ensure_config_dir(app: &AppHandle) -> Result<(), String> {
    let dir = config_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Result<Settings, String> {
    ensure_config_dir(&app)?;
    let path = settings_path(&app);
    if !path.exists() {
        let defaults = Settings::default();
        let json = serde_json::to_string_pretty(&defaults).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        return Ok(defaults);
    }

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    ensure_config_dir(&app)?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(&app), json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_shortcuts(app: AppHandle) -> Result<Vec<Shortcut>, String> {
    ensure_config_dir(&app)?;
    let path = shortcuts_path(&app);
    if !path.exists() {
        let defaults = default_shortcuts();
        let json = serde_json::to_string_pretty(&defaults).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        return Ok(defaults);
    }

    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_shortcuts(app: AppHandle, shortcuts: Vec<Shortcut>) -> Result<(), String> {
    ensure_config_dir(&app)?;
    let json = serde_json::to_string_pretty(&shortcuts).map_err(|e| e.to_string())?;
    std::fs::write(shortcuts_path(&app), json).map_err(|e| e.to_string())
}

fn default_shortcuts() -> Vec<Shortcut> {
    vec![
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+C".into(),
            action: "COPY".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+V".into(),
            action: "PASTE".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Tab".into(),
            action: "NEXT_TAB".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+Tab".into(),
            action: "PREVIOUS_TAB".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+S".into(),
            action: "SETTINGS".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+K".into(),
            action: "SHORTCUTS".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Ctrl+Shift+F".into(),
            action: "FUZZY_SEARCH".into(),
            enabled: true,
            linebreak: false,
        },
        Shortcut {
            shortcut_type: "app".into(),
            trigger: "Alt+Space".into(),
            action: "APP_LAUNCHER".into(),
            enabled: true,
            linebreak: false,
        },
    ]
}

#[tauri::command]
pub fn get_config_dir(app: AppHandle) -> Result<String, String> {
    ensure_config_dir(&app)?;
    Ok(config_dir(&app).to_string_lossy().into())
}
