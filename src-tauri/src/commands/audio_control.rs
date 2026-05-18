use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSink {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub volume_percent: u32,
    pub muted: bool,
    pub is_default: bool,
}

fn pactl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("pactl not found: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            "pactl command failed".to_string()
        } else {
            err
        })
    }
}

fn parse_percent_token(token: &str) -> Option<u32> {
    token
        .trim()
        .trim_end_matches('%')
        .parse::<u32>()
        .ok()
}

fn get_current_mute_state() -> Result<bool, String> {
    let out = pactl(&["get-sink-mute", "@DEFAULT_SINK@"]) ?;
    Ok(out.to_lowercase().contains("yes"))
}

#[tauri::command]
pub fn audio_available() -> bool {
    Command::new("pactl")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn list_audio_sinks() -> Result<Vec<AudioSink>, String> {
    let json_out = Command::new("pactl")
        .args(["--format=json", "list", "sinks"])
        .output()
        .map_err(|e| e.to_string())?;

    if !json_out.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&json_out.stdout);
    let sinks: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap_or_default();
    let default_sink = pactl(&["get-default-sink"]).unwrap_or_default();

    Ok(sinks
        .iter()
        .filter_map(|sink| {
            let index = sink.get("index")?.as_u64()? as u32;
            let name = sink.get("name")?.as_str()?.to_string();
            let description = sink
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            let muted = sink.get("mute").and_then(|v| v.as_bool()).unwrap_or(false);

            let volume_percent = sink
                .get("volume")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    let values: Vec<u32> = obj
                        .values()
                        .filter_map(|channel| channel.get("value_percent"))
                        .filter_map(|v| v.as_str())
                        .filter_map(parse_percent_token)
                        .collect();
                    if values.is_empty() {
                        100
                    } else {
                        values.iter().sum::<u32>() / values.len() as u32
                    }
                })
                .unwrap_or(100);

            Some(AudioSink {
                index,
                name: name.clone(),
                description,
                volume_percent,
                muted,
                is_default: name == default_sink,
            })
        })
        .collect())
}

#[tauri::command]
pub fn get_master_volume() -> Result<u32, String> {
    let wpctl = Command::new("wpctl").args(["get-volume", "@DEFAULT_AUDIO_SINK@"]).output();

    if let Ok(out) = wpctl {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(val_str) = text.split_whitespace().nth(1) {
                if let Ok(val) = val_str.parse::<f32>() {
                    return Ok((val * 100.0).round().clamp(0.0, 150.0) as u32);
                }
            }
        }
    }

    let out = pactl(&["get-sink-volume", "@DEFAULT_SINK@"]) ?;
    for part in out.split('/') {
        let trimmed = part.trim();
        if trimmed.ends_with('%') {
            if let Some(v) = parse_percent_token(trimmed) {
                return Ok(v);
            }
        }
    }

    Ok(100)
}

#[tauri::command]
pub fn set_master_volume(percent: u32) -> Result<(), String> {
    let percent = percent.min(150);
    let wpctl = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{}%", percent)])
        .status();
    if wpctl.map(|s| s.success()).unwrap_or(false) {
        return Ok(());
    }

    pactl(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", percent)])?;
    Ok(())
}

#[tauri::command]
pub fn toggle_mute() -> Result<bool, String> {
    let wpctl = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        .status();
    if wpctl.map(|s| s.success()).unwrap_or(false) {
        let out = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        return Ok(text.contains("[MUTED]"));
    }

    pactl(&["set-sink-mute", "@DEFAULT_SINK@", "toggle"])?;
    get_current_mute_state()
}

#[tauri::command]
pub fn set_default_sink(sink_name: String) -> Result<(), String> {
    pactl(&["set-default-sink", &sink_name])?;
    Ok(())
}
