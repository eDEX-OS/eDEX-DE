use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSettings {
    /// Seconds until screen turns off (0 = disabled)
    pub screen_timeout: u64,
    /// Seconds until system suspends (0 = disabled)
    pub suspend_timeout: u64,
    /// Whether suspend is enabled
    pub suspend_enabled: bool,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            screen_timeout: 300,
            suspend_timeout: 600,
            suspend_enabled: true,
        }
    }
}

fn power_settings_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("power.json")
}

#[command]
pub fn get_power_settings(app: AppHandle) -> PowerSettings {
    let path = power_settings_path(&app);
    if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        PowerSettings::default()
    }
}

#[command]
pub async fn set_power_settings(app: AppHandle, settings: PowerSettings) -> Result<(), String> {
    // Persist settings
    let path = power_settings_path(&app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())?;

    // Write a swayidle config and reload/restart swayidle
    apply_swayidle_config(&settings)?;
    Ok(())
}

fn apply_swayidle_config(settings: &PowerSettings) -> Result<(), String> {
    // Kill existing swayidle instance
    std::process::Command::new("pkill")
        .args(["-x", "swayidle"])
        .output()
        .ok();

    if !settings.suspend_enabled && settings.screen_timeout == 0 {
        // Nothing to do
        return Ok(());
    }

    let mut args: Vec<String> = vec!["-w".into()];

    if settings.screen_timeout > 0 {
        args.extend([
            "timeout".into(),
            settings.screen_timeout.to_string(),
            "hyprctl dispatch dpms off".into(),
            "resume".into(),
            "hyprctl dispatch dpms on".into(),
        ]);
    }

    if settings.suspend_enabled && settings.suspend_timeout > 0 {
        args.extend([
            "timeout".into(),
            settings.suspend_timeout.to_string(),
            "systemctl suspend".into(),
        ]);
    }

    std::process::Command::new("swayidle")
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to start swayidle: {e}"))?;

    Ok(())
}

/// Get battery information via upower.
#[command]
pub fn get_battery_status() -> serde_json::Value {
    let out = std::process::Command::new("upower")
        .args(["-i", "/org/freedesktop/UPower/devices/battery_BAT0"])
        .output();

    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut percentage = None::<f64>;
            let mut state = None::<String>;
            let mut time_to_empty = None::<String>;
            let mut time_to_full = None::<String>;

            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("percentage:") {
                    percentage = line
                        .split(':')
                        .nth(1)
                        .map(|s| s.trim().trim_end_matches('%'))
                        .and_then(|s| s.parse().ok());
                } else if line.starts_with("state:") {
                    state = line.split(':').nth(1).map(|s| s.trim().to_string());
                } else if line.starts_with("time to empty:") {
                    time_to_empty = line.split(':').nth(1).map(|s| s.trim().to_string());
                } else if line.starts_with("time to full:") {
                    time_to_full = line.split(':').nth(1).map(|s| s.trim().to_string());
                }
            }

            serde_json::json!({
                "present": true,
                "percentage": percentage,
                "state": state,
                "timeToEmpty": time_to_empty,
                "timeToFull": time_to_full,
            })
        }
        _ => serde_json::json!({ "present": false }),
    }
}
