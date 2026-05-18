use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemdUnit {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub description: String,
    pub unit_type: String,
    pub enabled: Option<bool>,
}

#[tauri::command]
pub fn list_units(
    user_units: bool,
    unit_type_filter: Option<String>,
) -> Result<Vec<SystemdUnit>, String> {
    let mut args = vec!["list-units", "--all", "--plain", "--no-legend", "--no-pager"];
    let type_arg;
    if let Some(ref filter) = unit_type_filter {
        type_arg = format!("--type={filter}");
        args.push(&type_arg);
    }

    let out = {
        let mut cmd = Command::new("systemctl");
        if user_units {
            cmd.arg("--user");
        }
        cmd.args(&args);
        cmd.output().map_err(|e| e.to_string())?
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let mut units: Vec<SystemdUnit> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                return None;
            }

            let name = parts[0].to_string();
            let unit_type = name.rsplit('.').next().unwrap_or("service").to_string();
            let description = parts[4..].join(" ");

            Some(SystemdUnit {
                name,
                load_state: parts[1].to_string(),
                active_state: parts[2].to_string(),
                sub_state: parts[3].to_string(),
                description,
                unit_type,
                enabled: None,
            })
        })
        .collect();

    units.sort_by(|a, b| {
        let priority = |state: &str| match state {
            "failed" => 0,
            "active" => 1,
            "activating" => 2,
            _ => 3,
        };
        priority(&a.active_state).cmp(&priority(&b.active_state))
    });

    Ok(units)
}

#[tauri::command]
pub fn unit_action(unit: String, action: String, user_units: bool) -> Result<String, String> {
    let action = match action.as_str() {
        "start" | "stop" | "restart" | "reload" | "enable" | "disable" | "mask" | "unmask" => {
            action
        }
        _ => return Err(format!("Invalid action: {action}")),
    };

    let out = if !user_units {
        let mut cmd = Command::new("pkexec");
        cmd.args(["systemctl", action.as_str(), &unit]);
        cmd.output().map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("systemctl");
        cmd.arg("--user");
        cmd.args([action.as_str(), &unit]);
        cmd.output().map_err(|e| e.to_string())?
    };

    if out.status.success() {
        Ok(format!("{} {}: OK", action, unit))
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("{} failed", action)
        } else {
            format!("{} failed: {}", action, stderr)
        })
    }
}

#[tauri::command]
pub fn get_unit_logs(unit: String, lines: Option<u32>) -> Result<String, String> {
    let count = lines.unwrap_or(50).min(500);
    let out = Command::new("journalctl")
        .args([
            "-u",
            &unit,
            "-n",
            &count.to_string(),
            "--no-pager",
            "--output=short-iso",
        ])
        .output()
        .map_err(|e| format!("journalctl not found: {e}"))?;

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[tauri::command]
pub fn get_unit_status(unit: String, user_units: bool) -> Result<String, String> {
    let mut cmd = Command::new("systemctl");
    if user_units {
        cmd.arg("--user");
    }
    cmd.args(["status", "--no-pager", "-l", &unit]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
