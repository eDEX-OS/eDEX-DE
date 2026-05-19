use super::{colors::Theme, resize::ResizeState};

#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Clone, Debug)]
pub struct StatusInfo {
    pub volume: u8,
    pub battery_pct: Option<u8>,
    pub battery_charging: bool,
    pub tor_active: bool,
    pub tailscale_active: bool,
    pub vpn_active: bool,
    pub net_tx_kbps: f32,
    pub net_rx_kbps: f32,
}

impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            volume: 42,
            battery_pct: Some(100),
            battery_charging: false,
            tor_active: false,
            tailscale_active: false,
            vpn_active: false,
            net_tx_kbps: 0.0,
            net_rx_kbps: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct UiState {
    pub clock: String,
    pub date: String,
    pub hostname: String,
    pub theme: &'static Theme,
    pub terminal_content: Vec<String>,
    pub filesystem_cwd: String,
    pub filesystem_entries: Vec<FsEntry>,
    pub selected_fs_entry: usize,
    pub boot_done: bool,
    pub boot_lines: Vec<String>,
    pub boot_overlay_alpha: f32,
    pub border_anim: f32,
    pub resize: ResizeState,
    pub tab_count: usize,
    pub active_tab: usize,
    pub status: StatusInfo,
}
