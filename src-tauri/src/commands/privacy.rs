use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("{cmd} not found: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// ── Tailscale structs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TailscalePeer {
    pub id: String,
    pub hostname: String,
    pub dns_name: String,
    pub os: String,
    pub tailscale_ips: Vec<String>,
    pub online: bool,
    pub exit_node_option: bool,
    pub exit_node: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleStatus {
    pub backend_state: String,
    pub version: String,
    pub self_node: Option<TailscalePeer>,
    pub peers: Vec<TailscalePeer>,
}

// ── Tor structs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TorStatus {
    pub active: bool,
    pub mode: String,
    pub socks_reachable: bool,
}

// ── VPN structs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnConnection {
    pub name: String,
    pub uuid: String,
    pub vpn_type: String,
    pub active: bool,
    pub device: String,
}

// ── Tailscale raw JSON shapes (for deserialization) ───────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawPeer {
    #[serde(rename = "ID")]
    id: Option<String>,
    host_name: Option<String>,
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "OS")]
    os: Option<String>,
    tailscale_i_ps: Option<Vec<String>>,
    online: Option<bool>,
    exit_node_option: Option<bool>,
    exit_node: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawTailscaleStatus {
    backend_state: Option<String>,
    version: Option<String>,
    #[serde(rename = "Self")]
    self_node: Option<RawPeer>,
    peer: Option<std::collections::HashMap<String, RawPeer>>,
}

fn raw_peer_to_struct(p: RawPeer) -> TailscalePeer {
    TailscalePeer {
        id: p.id.unwrap_or_default(),
        hostname: p.host_name.unwrap_or_default(),
        dns_name: p.dns_name.unwrap_or_default(),
        os: p.os.unwrap_or_default(),
        tailscale_ips: p.tailscale_i_ps.unwrap_or_default(),
        online: p.online.unwrap_or(false),
        exit_node_option: p.exit_node_option.unwrap_or(false),
        exit_node: p.exit_node.unwrap_or(false),
    }
}

// ── Tailscale commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn tailscale_available() -> bool {
    Command::new("tailscale")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn tailscale_status() -> Result<TailscaleStatus, String> {
    let out = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|e| format!("tailscale not found: {e}"))?;

    if !out.status.success() {
        return Ok(TailscaleStatus {
            backend_state: "Stopped".to_string(),
            version: String::new(),
            self_node: None,
            peers: vec![],
        });
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let raw: RawTailscaleStatus = serde_json::from_str(&text)
        .map_err(|e| format!("parse error: {e}"))?;

    let peers = raw
        .peer
        .unwrap_or_default()
        .into_values()
        .map(raw_peer_to_struct)
        .collect();

    Ok(TailscaleStatus {
        backend_state: raw.backend_state.unwrap_or_else(|| "Stopped".to_string()),
        version: raw.version.unwrap_or_default(),
        self_node: raw.self_node.map(raw_peer_to_struct),
        peers,
    })
}

#[tauri::command]
pub fn tailscale_login() -> Result<String, String> {
    use std::io::{BufRead, BufReader};
    use std::time::Instant;

    let mut child = Command::new("tailscale")
        .arg("login")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("tailscale not found: {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);

    // Search both stdout and stderr for the auth URL
    let reader_out = BufReader::new(stdout);
    let reader_err = BufReader::new(stderr);

    for line in reader_out.lines().chain(reader_err.lines()) {
        if Instant::now() > deadline {
            break;
        }
        if let Ok(l) = line {
            if l.contains("https://login.tailscale.com") {
                let _ = child.kill();
                return Ok(l.trim().to_string());
            }
        }
    }

    let _ = child.kill();
    Err("No auth URL found; tailscale may already be logged in".to_string())
}

#[tauri::command]
pub fn tailscale_logout() -> Result<(), String> {
    run("tailscale", &["logout"])?;
    Ok(())
}

#[tauri::command]
pub fn tailscale_up(exit_node: Option<String>) -> Result<(), String> {
    let mut args = vec!["up"];
    let node_arg;
    if let Some(ref ip) = exit_node {
        node_arg = format!("--exit-node={ip}");
        args.push(&node_arg);
    }
    run("tailscale", &args)?;
    Ok(())
}

#[tauri::command]
pub fn tailscale_down() -> Result<(), String> {
    run("tailscale", &["down"])?;
    Ok(())
}

#[tauri::command]
pub fn tailscale_set_exit_node(node_ip: Option<String>) -> Result<(), String> {
    let arg = match node_ip {
        Some(ref ip) => format!("--exit-node={ip}"),
        None => "--exit-node=".to_string(),
    };
    run("tailscale", &["set", &arg])?;
    Ok(())
}

// ── Tor commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn tor_available() -> bool {
    let out = Command::new("systemctl")
        .args(["status", "tor"])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).contains("tor.service"),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn tor_status() -> Result<TorStatus, String> {
    let active_out = Command::new("systemctl")
        .args(["is-active", "tor"])
        .output()
        .map_err(|e| e.to_string())?;
    let active = String::from_utf8_lossy(&active_out.stdout).trim() == "active";

    let mode = std::fs::read_to_string("/run/edex-tor-mode")
        .unwrap_or_else(|_| "off".to_string())
        .trim()
        .to_string();

    let socks_reachable = TcpStream::connect_timeout(
        &"127.0.0.1:9050".parse().unwrap(),
        Duration::from_millis(500),
    )
    .is_ok();

    Ok(TorStatus { active, mode, socks_reachable })
}

#[tauri::command]
pub fn tor_get_mode() -> Result<String, String> {
    Ok(std::fs::read_to_string("/run/edex-tor-mode")
        .unwrap_or_else(|_| "off".to_string())
        .trim()
        .to_string())
}

#[tauri::command]
pub fn tor_set_mode(mode: String) -> Result<(), String> {
    if !["off", "socks5", "transparent"].contains(&mode.as_str()) {
        return Err(format!("invalid tor mode: {mode}"));
    }
    if !std::path::Path::new("/usr/bin/edex-tor-mode").exists() {
        return Err("edex-tor-mode script not found at /usr/bin/edex-tor-mode".to_string());
    }
    let out = Command::new("pkexec")
        .args(["/usr/bin/edex-tor-mode", &mode])
        .output()
        .map_err(|e| format!("pkexec error: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[tauri::command]
pub fn tor_request_bridges(bridge_type: String) -> Result<Vec<String>, String> {
    let valid = ["obfs4", "snowflake", "vanilla"];
    if !valid.contains(&bridge_type.as_str()) {
        return Err(format!("invalid bridge type: {bridge_type}"));
    }
    let url = match bridge_type.as_str() {
        "obfs4" => "https://bridges.torproject.org/bridges?transport=obfs4",
        "snowflake" => "https://bridges.torproject.org/bridges?transport=snowflake",
        _ => "https://bridges.torproject.org/bridges",
    };
    let out = run("curl", &["-s", "--max-time", "15", url])?;
    let bridges = out
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    Ok(bridges)
}

#[tauri::command]
pub fn tor_get_bridges() -> Result<Vec<String>, String> {
    let path = "/etc/tor/torrc.d/bridges.conf";
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };
    Ok(content
        .lines()
        .filter(|l| l.starts_with("Bridge "))
        .map(|l| l.to_string())
        .collect())
}

#[tauri::command]
pub fn tor_set_bridges(bridges: Vec<String>) -> Result<(), String> {
    let content = if bridges.is_empty() {
        "UseBridges 0\n".to_string()
    } else {
        let mut s = "UseBridges 1\n".to_string();
        for b in &bridges {
            let line = b.strip_prefix("Bridge ").unwrap_or(b);
            s.push_str(&format!("Bridge {line}\n"));
        }
        s
    };

    std::fs::write("/tmp/edex-bridges.conf", &content)
        .map_err(|e| format!("write tmp: {e}"))?;

    let script = "mkdir -p /etc/tor/torrc.d && cp /tmp/edex-bridges.conf /etc/tor/torrc.d/bridges.conf && chmod 640 /etc/tor/torrc.d/bridges.conf && chown root:tor /etc/tor/torrc.d/bridges.conf";
    let out = Command::new("pkexec")
        .args(["sh", "-c", script])
        .output()
        .map_err(|e| format!("pkexec error: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let reload = Command::new("pkexec")
        .args(["systemctl", "reload-or-restart", "tor"])
        .output()
        .map_err(|e| format!("pkexec reload error: {e}"))?;
    if !reload.status.success() {
        return Err(String::from_utf8_lossy(&reload.stderr).trim().to_string());
    }
    Ok(())
}

// ── VPN commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn vpn_list_connections() -> Result<Vec<VpnConnection>, String> {
    let out = Command::new("nmcli")
        .args(["-t", "-f", "NAME,UUID,TYPE,DEVICE,STATE,ACTIVE", "connection", "show"])
        .output()
        .map_err(|_| "nmcli not found".to_string())?;

    if !out.status.success() {
        return Ok(vec![]);
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let conns = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(6, ':').collect();
            if parts.len() != 6 {
                return None;
            }
            let vpn_type = parts[2].to_string();
            if vpn_type != "wireguard" && vpn_type != "vpn" {
                return None;
            }
            Some(VpnConnection {
                name: parts[0].to_string(),
                uuid: parts[1].to_string(),
                vpn_type,
                device: parts[3].to_string(),
                active: parts[5].trim() == "yes",
            })
        })
        .collect();
    Ok(conns)
}

#[tauri::command]
pub fn vpn_connect(name: String) -> Result<(), String> {
    let out = Command::new("pkexec")
        .args(["nmcli", "connection", "up", &name])
        .output()
        .map_err(|e| format!("pkexec error: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[tauri::command]
pub fn vpn_disconnect(name: String) -> Result<(), String> {
    run("nmcli", &["connection", "down", &name])?;
    Ok(())
}

#[tauri::command]
pub fn vpn_import_wireguard(config_content: String, profile_name: String) -> Result<(), String> {
    let tmp = "/tmp/edex-wg-import.conf";
    std::fs::write(tmp, &config_content).map_err(|e| format!("write error: {e}"))?;

    let out = Command::new("pkexec")
        .args(["nmcli", "connection", "import", "type", "wireguard", "file", tmp])
        .output()
        .map_err(|e| format!("pkexec error: {e}"))?;

    let _ = std::fs::remove_file(tmp);

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.contains("wireguard") && err.contains("not supported") {
            return Err("WireGuard kernel module not loaded. Try: modprobe wireguard".to_string());
        }
        return Err(err);
    }

    // Rename the connection to the desired profile name
    let rename = Command::new("pkexec")
        .args(["nmcli", "connection", "modify", tmp, "connection.id", &profile_name])
        .output();
    // Best-effort rename; ignore errors
    let _ = rename;

    Ok(())
}
