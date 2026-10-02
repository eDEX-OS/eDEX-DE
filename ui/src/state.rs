//! The complete model the shell renders from.

use std::time::Instant;

use crate::{
    boot::BootAnimation,
    filesystem::FilesystemPanel,
    form::{Form, FormState},
    keyboard::KeyboardState,
    layout::{LayoutConfig, Metrics},
    terminal_model::{TabInfo, TerminalFrame},
    theme::Theme,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiskDisplay {
    pub mount: String,
    pub used_pct: f32,
    pub used_str: String,
    pub total_str: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuDisplay {
    pub name: String,
    pub busy_pct: Option<f32>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub temp_c: Option<f32>,
    pub power_w: Option<f32>,
    pub clock_mhz: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcDisplay {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_str: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SysInfo {
    pub cpu_cores: Vec<f32>,
    pub cpu_model: String,
    pub cpu_freq_mhz: u64,
    pub cpu_temp_c: Option<f32>,
    pub load_avg: [f32; 3],
    pub uptime_secs: u64,
    pub kernel: String,
    pub ram_used_kb: u64,
    pub ram_total_kb: u64,
    pub ram_cached_kb: u64,
    pub gpus: Vec<GpuDisplay>,
    /// Temperature sensors (label, °C), hottest first.
    pub temps: Vec<(String, f32)>,
    pub swap_used_kb: u64,
    pub swap_total_kb: u64,
    pub net_tx_history: Vec<f32>,
    pub net_rx_history: Vec<f32>,
    pub net_tx_kbps: f32,
    pub net_rx_kbps: f32,
    pub net_iface: String,
    pub net_ip: String,
    pub disks: Vec<DiskDisplay>,
    pub processes: Vec<ProcDisplay>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusInfo {
    pub volume: Option<u8>,
    pub muted: bool,
    pub mic_muted: bool,
    pub battery_pct: Option<u8>,
    pub battery_charging: bool,
    pub tor_mode: String,
    pub tor_active: bool,
    pub tailscale_active: bool,
    pub vpn_active: bool,
    pub wireguard_active: bool,
    pub fprintd_active: bool,
    pub mic_active: bool,
    pub camera_active: bool,
    pub wifi_ssid: Option<String>,
    pub ethernet: bool,
    pub bluetooth_on: bool,
    pub bluetooth_connected: usize,
    pub unread_notifications: usize,
    pub dnd: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub windows: u32,
}

/// An application window shown as a tab in the centre panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowTab {
    pub address: String,
    pub class: String,
    pub title: String,
    /// Focused window.
    pub active: bool,
    /// Parked on the minimized workspace; clicking the tab brings it back.
    pub minimized: bool,
    pub maximized: bool,
    pub floating: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PanelFocus {
    #[default]
    Terminal,
    Filesystem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    Launcher,
    Settings,
    Privacy,
    Notifications,
    Power,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LauncherState {
    pub query: String,
    pub results: Vec<LauncherResult>,
    pub selected: usize,
    pub scroll: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LauncherResult {
    pub name: String,
    pub comment: Option<String>,
    pub category: Option<String>,
}

impl LauncherState {
    pub fn reset(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Lock,
    Logout,
    Suspend,
    Hibernate,
    Reboot,
    PowerOff,
}

impl PowerAction {
    pub const ALL: [PowerAction; 6] = [
        PowerAction::Lock,
        PowerAction::Logout,
        PowerAction::Suspend,
        PowerAction::Hibernate,
        PowerAction::Reboot,
        PowerAction::PowerOff,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PowerAction::Lock => "LOCK",
            PowerAction::Logout => "LOG OUT",
            PowerAction::Suspend => "SUSPEND",
            PowerAction::Hibernate => "HIBERNATE",
            PowerAction::Reboot => "REBOOT",
            PowerAction::PowerOff => "POWER OFF",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            PowerAction::Lock => "⌧",
            PowerAction::Logout => "⇦",
            PowerAction::Suspend => "☾",
            PowerAction::Hibernate => "❄",
            PowerAction::Reboot => "↻",
            PowerAction::PowerOff => "⏻",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerMenuState {
    pub selected: usize,
    pub available: Vec<PowerAction>,
    /// Action awaiting confirmation (reboot/power off).
    pub confirm: Option<PowerAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastView {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    pub urgency: u8,
    pub progress: f32,
    pub actions: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Osd {
    pub label: String,
    pub value: f32,
    pub muted: bool,
    pub shown_at: Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotificationRow {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    pub time: String,
    pub urgency: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotificationsView {
    pub rows: Vec<NotificationRow>,
    pub selected: usize,
    pub dnd: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerminalView {
    pub tabs: Vec<TabInfo>,
    pub active: usize,
    pub frame: TerminalFrame,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResizeState {
    pub dragging: Option<crate::hit::ResizeHandle>,
    pub hover: Option<crate::hit::ResizeHandle>,
}

/// Tabbed form-based overlay (settings, privacy).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabbedForms {
    pub tabs: Vec<String>,
    pub active: usize,
    pub form: Form,
    pub form_state: FormState,
    pub title: String,
    pub status: Option<String>,
}

pub struct ShellState {
    pub theme: Theme,
    pub metrics: Metrics,
    pub layout_cfg: LayoutConfig,
    pub scanlines: bool,
    pub animations: bool,
    pub hostname: String,
    pub username: String,
    pub clock: String,
    pub date: String,
    pub workspaces: Vec<WorkspaceInfo>,
    pub active_window: Option<String>,
    /// Windows on the visible workspace, then minimized ones (centre panel tabs).
    pub windows: Vec<WindowTab>,
    /// Tiled windows cover the terminal on the visible workspace.
    pub apps_cover_terminal: bool,
    /// Side panels are not reserved and apps are open: the tab strip spans the full width.
    pub wide_tab_strip: bool,
    pub kb_layout: String,
    pub hypr_connected: bool,
    pub live_iso: bool,
    pub sysinfo: SysInfo,
    pub status: StatusInfo,
    pub terminal: TerminalView,
    pub filesystem: FilesystemPanel,
    pub keyboard: KeyboardState,
    pub focus: PanelFocus,
    pub shell_focused: bool,
    pub resize: ResizeState,
    pub boot: BootAnimation,
    pub now: Instant,
    pub overlay: Option<OverlayKind>,
    pub launcher: LauncherState,
    pub power: PowerMenuState,
    pub settings: TabbedForms,
    pub privacy: TabbedForms,
    pub notifications: NotificationsView,
    pub osd: Option<Osd>,
    pub toasts: Vec<ToastView>,
    pub version: String,
}

impl ShellState {
    pub fn new(theme: Theme, metrics: Metrics) -> Self {
        Self {
            theme,
            metrics,
            layout_cfg: LayoutConfig::default(),
            scanlines: true,
            animations: true,
            hostname: String::from("edex"),
            username: String::new(),
            clock: String::new(),
            date: String::new(),
            workspaces: Vec::new(),
            active_window: None,
            windows: Vec::new(),
            apps_cover_terminal: false,
            wide_tab_strip: false,
            kb_layout: String::from("us"),
            hypr_connected: false,
            live_iso: false,
            sysinfo: SysInfo::default(),
            status: StatusInfo::default(),
            terminal: TerminalView::default(),
            filesystem: FilesystemPanel::new(),
            keyboard: KeyboardState::default(),
            focus: PanelFocus::Terminal,
            shell_focused: false,
            resize: ResizeState::default(),
            boot: BootAnimation::default(),
            now: Instant::now(),
            overlay: None,
            launcher: LauncherState::default(),
            power: PowerMenuState {
                selected: 0,
                available: PowerAction::ALL.to_vec(),
                confirm: None,
            },
            settings: TabbedForms::default(),
            privacy: TabbedForms::default(),
            notifications: NotificationsView::default(),
            osd: None,
            toasts: Vec::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Border pulse phase in 0..1 derived from wall-clock time. Quantised to 32 levels so the
    /// shell only redraws when the glow visibly changes (~8 times a second, not every tick).
    pub fn pulse(&self) -> f32 {
        if !self.animations {
            return 0.5;
        }
        let t = self.now.duration_since(self.boot_start()).as_secs_f32();
        (((t * 0.8).sin() * 0.5 + 0.5) * 32.0).round() / 32.0
    }

    fn boot_start(&self) -> Instant {
        self.now - self.boot.elapsed(self.now)
    }
}
