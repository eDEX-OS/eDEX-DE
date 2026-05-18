use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub device_type: String,
}

fn run_bluetoothctl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("bluetoothctl")
        .args(args)
        .output()
        .map_err(|e| format!("bluetoothctl not found: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[command]
pub fn bluetooth_available() -> bool {
    std::process::Command::new("bluetoothctl")
        .arg("show")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[command]
pub fn bluetooth_list_devices() -> Result<Vec<BluetoothDevice>, String> {
    let paired_out = run_bluetoothctl(&["devices", "Paired"])?;
    let connected_out = run_bluetoothctl(&["devices", "Connected"])?;

    let connected_addrs: std::collections::HashSet<&str> = connected_out
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    let mut devices = Vec::new();
    for line in paired_out.lines() {
        // Format: "Device AA:BB:CC:DD:EE:FF Device Name"
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() < 3 || parts[0] != "Device" {
            continue;
        }
        let address = parts[1].to_string();
        let name = parts[2].to_string();
        let connected = connected_addrs.contains(address.as_str());
        devices.push(BluetoothDevice {
            address: address.clone(),
            name,
            paired: true,
            connected,
            device_type: get_device_type(&address),
        });
    }
    Ok(devices)
}

#[command]
pub async fn bluetooth_scan() -> Result<Vec<BluetoothDevice>, String> {
    // Start discovery for 5 seconds
    std::process::Command::new("bluetoothctl")
        .args(["--timeout", "5", "scan", "on"])
        .output()
        .map_err(|e| e.to_string())?;

    let out = run_bluetoothctl(&["devices"])?;
    let paired_out = run_bluetoothctl(&["devices", "Paired"])?;
    let paired_addrs: std::collections::HashSet<&str> = paired_out
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    let mut devices = Vec::new();
    for line in out.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() < 3 || parts[0] != "Device" {
            continue;
        }
        let address = parts[1].to_string();
        let name = parts[2].to_string();
        let paired = paired_addrs.contains(address.as_str());
        devices.push(BluetoothDevice {
            address: address.clone(),
            name,
            paired,
            connected: false,
            device_type: get_device_type(&address),
        });
    }
    Ok(devices)
}

#[command]
pub async fn bluetooth_connect(address: String) -> Result<(), String> {
    let out = std::process::Command::new("bluetoothctl")
        .args(["connect", &address])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[command]
pub async fn bluetooth_disconnect(address: String) -> Result<(), String> {
    let out = std::process::Command::new("bluetoothctl")
        .args(["disconnect", &address])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[command]
pub async fn bluetooth_pair(address: String) -> Result<(), String> {
    let out = std::process::Command::new("bluetoothctl")
        .args(["pair", &address])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[command]
pub async fn bluetooth_remove(address: String) -> Result<(), String> {
    let out = std::process::Command::new("bluetoothctl")
        .args(["remove", &address])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

fn get_device_type(address: &str) -> String {
    // Try to read icon/type from bluetoothctl info
    let info = std::process::Command::new("bluetoothctl")
        .args(["info", address])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();

    for line in info.lines() {
        let l = line.trim();
        if l.starts_with("Icon:") {
            let icon = l.trim_start_matches("Icon:").trim();
            return match icon {
                "audio-headset" | "audio-headphones" => "headphones".into(),
                "input-keyboard" => "keyboard".into(),
                "input-mouse" => "mouse".into(),
                "phone" => "phone".into(),
                _ => icon.to_string(),
            };
        }
    }
    "device".into()
}
