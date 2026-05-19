//! Privacy status checks — VPN, Tor, fingerprint, mic/camera.

use std::process::Command;

#[derive(Clone, Debug, Default)]
pub struct PrivacyStatus {
    pub tailscale_connected: bool,
    pub wireguard_active: bool,
    pub tor_active: bool,
    pub fprintd_active: bool,
    pub mic_active: bool,
    pub camera_active: bool,
}

pub fn get_privacy_status() -> PrivacyStatus {
    PrivacyStatus {
        tailscale_connected: check_tailscale(),
        wireguard_active: check_wireguard(),
        tor_active: check_service("tor"),
        fprintd_active: check_service("fprintd"),
        mic_active: check_mic(),
        camera_active: check_camera(),
    }
}

fn check_tailscale() -> bool {
    Command::new("tailscale")
        .arg("status")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_wireguard() -> bool {
    // Check /proc/net/dev for any wg* interface
    if let Ok(contents) = std::fs::read_to_string("/proc/net/dev") {
        for line in contents.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("wg") {
                return true;
            }
        }
    }
    // Fallback: ip link
    Command::new("ip")
        .args(["link", "show", "type", "wireguard"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

fn check_service(name: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_mic() -> bool {
    // Check if any /dev/snd/pcmC*D*c (capture) device is open via /proc/*/fd
    check_dev_in_use("/dev/snd/", "c")
}

fn check_camera() -> bool {
    // Check if /dev/video* is open by any process
    check_dev_in_use("/dev/video", "")
}

fn check_dev_in_use(prefix: &str, suffix: &str) -> bool {
    let proc = match std::fs::read_dir("/proc") {
        Ok(d) => d,
        Err(_) => return false,
    };
    for entry in proc.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Only numeric directories (PIDs)
        if !name_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let fd_path = entry.path().join("fd");
        let fds = match std::fs::read_dir(&fd_path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for fd in fds.flatten() {
            if let Ok(target) = std::fs::read_link(fd.path()) {
                let t = target.to_string_lossy();
                if t.starts_with(prefix) && (suffix.is_empty() || t.ends_with(suffix)) {
                    return true;
                }
            }
        }
    }
    false
}
