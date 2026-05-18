use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

fn socket_path(socket: &str) -> Option<PathBuf> {
    let sig = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    Some(PathBuf::from(format!("/tmp/hypr/{sig}/{socket}")))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub monitor: String,
    pub windows: u32,
    pub has_fullscreen: bool,
    pub last_window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindowInfo {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace_id: i32,
    pub workspace_name: String,
    pub monitor: String,
    pub pid: u32,
    pub floating: bool,
    pub width: u32,
    pub height: u32,
    pub at: [i32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub active_workspace_id: i32,
    pub active_workspace_name: String,
    pub focused: bool,
}

async fn hypr_command(cmd: &str) -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let path = socket_path(".socket.sock")
        .ok_or_else(|| "HYPRLAND_INSTANCE_SIGNATURE not set — is Hyprland running?".to_string())?;

    let mut stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("Hyprland socket error: {e}"))?;

    let mut command = cmd.to_string();
    if !command.ends_with('\n') {
        command.push('\n');
    }

    stream
        .write_all(command.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.shutdown().await.map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&buf).trim().to_string())
}

pub async fn get_workspaces() -> Result<Vec<WorkspaceInfo>, String> {
    let json = hypr_command("j/workspaces").await?;
    let raw: Vec<serde_json::Value> = serde_json::from_str(&json)
        .map_err(|e| format!("JSON parse error: {e}\nRaw: {json}"))?;

    Ok(raw
        .iter()
        .filter_map(|v| {
            Some(WorkspaceInfo {
                id: v["id"].as_i64()? as i32,
                name: v["name"].as_str()?.to_string(),
                monitor: v["monitor"].as_str().unwrap_or("").to_string(),
                windows: v["windows"].as_u64().unwrap_or(0) as u32,
                has_fullscreen: v["hasfullscreen"].as_bool().unwrap_or(false),
                last_window: v["lastwindow"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect())
}

pub async fn get_active_window() -> Result<Option<ActiveWindowInfo>, String> {
    let json = hypr_command("j/activewindow").await?;
    if json.trim().is_empty() || json.trim() == "{}" {
        return Ok(None);
    }

    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| format!("JSON parse: {e}"))?;

    Ok(Some(ActiveWindowInfo {
        address: v["address"].as_str().unwrap_or("").to_string(),
        class: v["class"].as_str().unwrap_or("").to_string(),
        title: v["title"].as_str().unwrap_or("").to_string(),
        workspace_id: v["workspace"]["id"].as_i64().unwrap_or(0) as i32,
        workspace_name: v["workspace"]["name"].as_str().unwrap_or("").to_string(),
        monitor: v["monitor"].as_str().unwrap_or("").to_string(),
        pid: v["pid"].as_u64().unwrap_or(0) as u32,
        floating: v["floating"].as_bool().unwrap_or(false),
        width: v["size"][0].as_u64().unwrap_or(0) as u32,
        height: v["size"][1].as_u64().unwrap_or(0) as u32,
        at: [
            v["at"][0].as_i64().unwrap_or(0) as i32,
            v["at"][1].as_i64().unwrap_or(0) as i32,
        ],
    }))
}

pub async fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    let json = hypr_command("j/monitors").await?;
    let raw: Vec<serde_json::Value> = serde_json::from_str(&json).map_err(|e| format!("JSON parse: {e}"))?;

    Ok(raw
        .iter()
        .filter_map(|v| {
            Some(MonitorInfo {
                id: v["id"].as_u64()? as u32,
                name: v["name"].as_str()?.to_string(),
                description: v["description"].as_str().unwrap_or("").to_string(),
                width: v["width"].as_u64().unwrap_or(0) as u32,
                height: v["height"].as_u64().unwrap_or(0) as u32,
                refresh_rate: v["refreshRate"].as_f64().unwrap_or(60.0) as f32,
                x: v["x"].as_i64().unwrap_or(0) as i32,
                y: v["y"].as_i64().unwrap_or(0) as i32,
                active_workspace_id: v["activeWorkspace"]["id"].as_i64().unwrap_or(0) as i32,
                active_workspace_name: v["activeWorkspace"]["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                focused: v["focused"].as_bool().unwrap_or(false),
            })
        })
        .collect())
}

pub async fn dispatch(action: &str) -> Result<String, String> {
    hypr_command(&format!("dispatch {action}")).await
}

pub async fn switch_workspace(id: i32) -> Result<(), String> {
    dispatch(&format!("workspace {id}")).await.map(|_| ())
}

pub fn is_hyprland_running() -> bool {
    env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}

pub async fn start_event_listener(app_handle: tauri::AppHandle) {
    use tauri::Emitter;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::net::UnixStream;

    let Some(path) = socket_path(".socket2.sock") else {
        return;
    };

    loop {
        let stream = match UnixStream::connect(&path).await {
            Ok(stream) => stream,
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
        };

        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Some((event, data)) = line.split_once(">>") {
                match event {
                    "workspace"
                    | "focusedmon"
                    | "activewindow"
                    | "openwindow"
                    | "closewindow"
                    | "movewindow"
                    | "activelayout"
                    | "createworkspace"
                    | "destroyworkspace"
                    | "moveworkspace" => {
                        let _ = app_handle.emit(
                            "hyprland-event",
                            serde_json::json!({
                                "event": event,
                                "data": data,
                            }),
                        );
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}
