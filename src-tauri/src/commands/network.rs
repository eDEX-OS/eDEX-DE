use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConnection {
    pub name: String,
    pub uuid: String,
    pub conn_type: String,
    pub device: String,
    pub active: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal: u8,
    pub security: String,
    pub in_use: bool,
    pub freq: String,
}

fn nmcli(args: &[&str]) -> Result<String, String> {
    let out = Command::new("nmcli")
        .args(args)
        .output()
        .map_err(|e| format!("nmcli not found: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            "nmcli command failed".to_string()
        } else {
            err
        })
    }
}

#[tauri::command]
pub fn network_available() -> bool {
    Command::new("nmcli")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn list_connections() -> Result<Vec<NetworkConnection>, String> {
    let out = nmcli(&["-t", "-f", "NAME,UUID,TYPE,DEVICE,STATE,ACTIVE", "connection", "show"])?;
    Ok(out
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(6, ':').collect();
            if parts.len() != 6 {
                return None;
            }

            Some(NetworkConnection {
                name: parts[0].to_string(),
                uuid: parts[1].to_string(),
                conn_type: parts[2].to_string(),
                device: parts[3].to_string(),
                state: parts[4].to_string(),
                active: parts[5].trim() == "yes",
            })
        })
        .collect())
}

#[tauri::command]
pub fn wifi_scan() -> Result<Vec<WifiNetwork>, String> {
    let _ = Command::new("nmcli").args(["device", "wifi", "rescan"]).status();
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let out = Command::new("nmcli")
        .args([
            "--escape",
            "no",
            "-g",
            "IN-USE,SSID,BSSID,SIGNAL,SECURITY,FREQ",
            "device",
            "wifi",
            "list",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut seen = HashSet::new();
    let mut networks: Vec<WifiNetwork> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.rsplitn(4, ':').collect();
            if parts.len() != 4 {
                return None;
            }

            let tail = parts[3];
            let freq = parts[0].trim().to_string();
            let security = parts[1].trim().to_string();
            let signal = parts[2].trim().parse::<u8>().unwrap_or(0);

            let mut front = tail.splitn(3, ':');
            let in_use = front.next().unwrap_or("").trim() == "*";
            let ssid = front.next().unwrap_or("").trim().to_string();
            let bssid = front.next().unwrap_or("").trim().to_string();

            if ssid.is_empty() || !seen.insert(ssid.clone()) {
                return None;
            }

            Some(WifiNetwork {
                ssid,
                bssid,
                signal,
                security,
                in_use,
                freq,
            })
        })
        .collect();

    networks.sort_by(|a, b| b.signal.cmp(&a.signal));
    Ok(networks)
}

#[tauri::command]
pub fn wifi_connect(ssid: String, password: Option<String>) -> Result<(), String> {
    let mut args = vec!["device", "wifi", "connect", ssid.as_str()];
    if let Some(ref pw) = password {
        args.extend_from_slice(&["password", pw.as_str()]);
    }
    nmcli(&args)?;
    Ok(())
}

#[tauri::command]
pub fn nm_disconnect(device: String) -> Result<(), String> {
    nmcli(&["device", "disconnect", &device])?;
    Ok(())
}

#[tauri::command]
pub fn get_active_connection_info() -> Result<serde_json::Value, String> {
    let out = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "GENERAL.CONNECTION,GENERAL.DEVICE,GENERAL.STATE,IP4.ADDRESS,IP4.GATEWAY,GENERAL.TYPE",
            "device",
            "show",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let text = String::from_utf8_lossy(&out.stdout);
    let mut info = serde_json::json!({});

    for line in text.lines() {
        if let Some((key, val)) = line.split_once(':') {
            match key.trim() {
                "GENERAL.CONNECTION" => {
                    if info["connection"].is_null() && !val.trim().is_empty() {
                        info["connection"] = serde_json::json!(val.trim());
                    }
                }
                "GENERAL.DEVICE" => {
                    if info["device"].is_null() && !val.trim().is_empty() {
                        info["device"] = serde_json::json!(val.trim());
                    }
                }
                "GENERAL.STATE" => {
                    if info["state"].is_null() && !val.trim().is_empty() {
                        info["state"] = serde_json::json!(val.trim());
                    }
                }
                "IP4.ADDRESS[1]" => {
                    if info["ipv4"].is_null() && !val.trim().is_empty() {
                        info["ipv4"] = serde_json::json!(val.trim());
                    }
                }
                "IP4.GATEWAY" => {
                    if info["gateway"].is_null() && !val.trim().is_empty() {
                        info["gateway"] = serde_json::json!(val.trim());
                    }
                }
                "GENERAL.TYPE" => {
                    if info["connType"].is_null() && !val.trim().is_empty() {
                        info["connType"] = serde_json::json!(val.trim());
                    }
                }
                _ => {}
            }
        }
    }

    Ok(info)
}
