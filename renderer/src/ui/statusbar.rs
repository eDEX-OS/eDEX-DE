use super::state::StatusInfo;

pub fn compose(status: &StatusInfo) -> String {
    let mut privacy = Vec::new();
    if status.tor_active {
        privacy.push("🧅");
    }
    if status.tailscale_active {
        privacy.push("🔒");
    }
    if status.vpn_active {
        privacy.push("🔐");
    }

    let mut parts = Vec::new();
    if !privacy.is_empty() {
        parts.push(privacy.join(" "));
    }

    parts.push(format!("🔊 {}%", status.volume));

    if let Some(battery_pct) = status.battery_pct {
        let icon = if status.battery_charging {
            "⚡"
        } else {
            "🔋"
        };
        parts.push(format!("{} {}%", icon, battery_pct));
    }

    parts.push(format!("▲ {:.1} kb/s", status.net_tx_kbps));
    parts.push(format!("▼ {:.1} kb/s", status.net_rx_kbps));

    parts.join("   ")
}
