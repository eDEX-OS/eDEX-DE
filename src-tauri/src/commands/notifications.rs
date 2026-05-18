use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub font: String,
    pub font_size: u32,
    pub background_color: String,
    pub text_color: String,
    pub border_color: String,
    pub timeout: u32,
    pub max_visible: u32,
    pub position: String,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            font: "monospace".into(),
            font_size: 12,
            background_color: "#1a1a2e".into(),
            text_color: "#00ff99".into(),
            border_color: "#00ff9966".into(),
            timeout: 5000,
            max_visible: 5,
            position: "top-right".into(),
        }
    }
}

fn mako_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("mako/config")
}

#[command]
pub fn get_notification_config() -> NotificationConfig {
    let path = mako_config_path();
    if !path.exists() {
        return NotificationConfig::default();
    }
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    parse_mako_config(&content)
}

fn parse_mako_config(content: &str) -> NotificationConfig {
    let mut cfg = NotificationConfig::default();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "font" => {
                    // "monospace 12" or "monospace"
                    let parts: Vec<&str> = v.trim().rsplitn(2, ' ').collect();
                    if parts.len() == 2 {
                        if let Ok(sz) = parts[0].parse::<u32>() {
                            cfg.font_size = sz;
                            cfg.font = parts[1].to_string();
                        } else {
                            cfg.font = v.trim().to_string();
                        }
                    } else {
                        cfg.font = v.trim().to_string();
                    }
                }
                "background-color" => cfg.background_color = v.trim().to_string(),
                "color" => cfg.text_color = v.trim().to_string(),
                "border-color" => cfg.border_color = v.trim().to_string(),
                "default-timeout" => {
                    cfg.timeout = v.trim().parse().unwrap_or(5000);
                }
                "max-visible" => {
                    cfg.max_visible = v.trim().parse().unwrap_or(5);
                }
                "anchor" => {
                    cfg.position = match v.trim() {
                        "top-right" => "top-right",
                        "top-center" => "top-center",
                        "top-left" => "top-left",
                        "bottom-right" => "bottom-right",
                        "bottom-center" => "bottom-center",
                        "bottom-left" => "bottom-left",
                        other => other,
                    }
                    .to_string();
                }
                _ => {}
            }
        }
    }
    cfg
}

#[command]
pub async fn set_notification_config(config: NotificationConfig) -> Result<(), String> {
    let path = mako_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = format!(
        "# eDEX-DE notification config (managed by settings panel)\n\
         font={font} {font_size}\n\
         background-color={bg}\n\
         color={text}\n\
         border-color={border}\n\
         border-size=2\n\
         border-radius=4\n\
         default-timeout={timeout}\n\
         max-visible={max_visible}\n\
         anchor={position}\n\
         layer=overlay\n\
         margin=8\n\
         padding=8,16\n",
        font = config.font,
        font_size = config.font_size,
        bg = config.background_color,
        text = config.text_color,
        border = config.border_color,
        timeout = config.timeout,
        max_visible = config.max_visible,
        position = config.position,
    );

    std::fs::write(&path, content).map_err(|e| e.to_string())?;

    // Reload mako if it's running
    std::process::Command::new("makoctl")
        .arg("reload")
        .output()
        .ok();

    Ok(())
}
