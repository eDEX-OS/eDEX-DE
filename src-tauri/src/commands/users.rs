use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    pub groups: Vec<String>,
}

/// List all non-system users (UID >= 1000) from /etc/passwd.
#[command]
pub fn list_users() -> Vec<UserInfo> {
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut users = Vec::new();

    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 7 {
            continue;
        }
        let uid: u32 = fields[2].parse().unwrap_or(0);
        if uid < 1000 || uid == 65534 {
            continue; // skip system users and nobody
        }
        let username = fields[0].to_string();
        let groups = get_user_groups(&username);
        users.push(UserInfo {
            username,
            uid,
            gid: fields[3].parse().unwrap_or(0),
            home: fields[5].to_string(),
            shell: fields[6].to_string(),
            groups,
        });
    }
    users
}

fn get_user_groups(username: &str) -> Vec<String> {
    let out = std::process::Command::new("groups")
        .arg(username)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    // "username : group1 group2 ..."
    out.splitn(2, ':')
        .nth(1)
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Change the current user's password.
/// Spawns `passwd` with the old and new passwords piped in.
#[command]
pub async fn change_password(
    username: String,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    // Use chpasswd for a non-interactive password change (requires root/sudo)
    // For the current user, we use passwd via expect-style input
    let input = format!("{old_password}\n{new_password}\n{new_password}\n");
    let mut child = std::process::Command::new("passwd")
        .arg(&username)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn passwd: {e}"))?;

    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .into())
    }
}
