pub fn read_battery() -> Option<(u8, bool)> {
    for bat in &["BAT0", "BAT1"] {
        let cap_path = format!("/sys/class/power_supply/{}/capacity", bat);
        let status_path = format!("/sys/class/power_supply/{}/status", bat);
        if let Ok(cap_str) = std::fs::read_to_string(&cap_path) {
            let pct = cap_str.trim().parse::<u8>().ok()?;
            let charging = std::fs::read_to_string(&status_path)
                .map(|s| matches!(s.trim(), "Charging" | "Full"))
                .unwrap_or(false);
            return Some((pct, charging));
        }
    }
    None
}
