use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub enabled: bool,
}

/// Returns current monitor layout via `hyprctl monitors -j`.
#[command]
pub fn get_display_info() -> Result<Vec<serde_json::Value>, String> {
    let out = std::process::Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|e| format!("hyprctl not found: {e}"))?;
    let json: Vec<serde_json::Value> =
        serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    Ok(json)
}

/// Apply a monitor config change via `hyprctl keyword monitor`.
/// Format: "name,WxH@R,position,scale"  e.g. "DP-1,1920x1080@60,0x0,1"
#[command]
pub async fn set_monitor_config(config: MonitorConfig) -> Result<(), String> {
    let spec = format!(
        "{},{width}x{height}@{rate},{x}x{y},{scale}",
        config.name,
        width = config.width,
        height = config.height,
        rate = config.refresh_rate,
        x = config.x,
        y = config.y,
        scale = config.scale,
    );
    let out = std::process::Command::new("hyprctl")
        .args(["keyword", "monitor", &spec])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// Set keyboard layout via hyprctl.
#[command]
pub async fn set_keyboard_layout(layout: String) -> Result<(), String> {
    let out = std::process::Command::new("hyprctl")
        .args(["keyword", "input:kb_layout", &layout])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// Set mouse sensitivity (-1.0 to 1.0) via hyprctl.
#[command]
pub async fn set_mouse_sensitivity(sensitivity: f64) -> Result<(), String> {
    let val = sensitivity.to_string();
    let out = std::process::Command::new("hyprctl")
        .args(["keyword", "input:sensitivity", &val])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

/// Toggle touchpad natural scroll via hyprctl.
#[command]
pub async fn set_natural_scroll(enabled: bool) -> Result<(), String> {
    let val = if enabled { "true" } else { "false" };
    let out = std::process::Command::new("hyprctl")
        .args(["keyword", "input:touchpad:natural_scroll", val])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}
