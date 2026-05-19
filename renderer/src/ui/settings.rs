//! eDEX-DE settings panel — full-screen overlay with 14 categories.

pub const CATEGORIES: &[&str] = &[
    "Appearance",
    "Display",
    "Input",
    "Audio",
    "Network",
    "Bluetooth",
    "Power",
    "Security",
    "Users",
    "Notifications",
    "Services",
    "Compositor",
    "Terminal",
    "About",
];

// ── Per-category settings structs ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppearanceSettings {
    pub theme_index: usize,   // 0=tron, 1=matrix, 2=amber
    pub border_glow: f32,     // 0.0–1.0
    pub font_size: u32,       // 10–24
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme_index: 0,
            border_glow: 0.7,
            font_size: 14,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplaySettings {
    pub scale: f32,
    pub refresh_rate: u32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            refresh_rate: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputSettings {
    pub kb_layout: String,
    pub mouse_sensitivity: f32,
    pub scroll_speed: f32,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            kb_layout: "us".to_string(),
            mouse_sensitivity: 1.0,
            scroll_speed: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub volume: u32,   // 0–100
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 70,
            muted: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkSettings {
    pub wifi_enabled: bool,
    pub active_connection: String,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            wifi_enabled: true,
            active_connection: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BluetoothSettings {
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PowerSettings {
    pub screen_blank_secs: u32,
    pub suspend_secs: u32,
    pub hibernate_on_low: bool,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            screen_blank_secs: 300,
            suspend_secs: 900,
            hibernate_on_low: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecuritySettings {
    pub lock_timeout_secs: u32,
    pub tailscale_enabled: bool,
    pub wireguard_enabled: bool,
    pub tor_enabled: bool,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            lock_timeout_secs: 600,
            tailscale_enabled: false,
            wireguard_enabled: false,
            tor_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UsersSettings {
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct NotificationsSettings {
    pub do_not_disturb: bool,
    pub history_limit: usize,
}

impl Default for NotificationsSettings {
    fn default() -> Self {
        Self {
            do_not_disturb: false,
            history_limit: 50,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ServicesSettings {
    pub service_list: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompositorSettings {
    pub border_width: u32,
    pub gap_size: u32,
    pub animation_speed: f32,
}

impl Default for CompositorSettings {
    fn default() -> Self {
        Self {
            border_width: 2,
            gap_size: 8,
            animation_speed: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalSettings {
    pub shell: String,
    pub scrollback: u32,
    pub cursor_style: usize,   // 0=block, 1=underline, 2=bar
    pub font_size: u32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            scrollback: 10_000,
            cursor_style: 0,
            font_size: 13,
        }
    }
}

// ── Main panel ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SettingsPanel {
    pub active: bool,
    pub selected_category: usize,
    pub focus_right: bool,   // false = sidebar focused, true = content pane focused

    // Per-category state
    pub appearance: AppearanceSettings,
    pub display: DisplaySettings,
    pub input: InputSettings,
    pub audio: AudioSettings,
    pub network: NetworkSettings,
    pub bluetooth: BluetoothSettings,
    pub power: PowerSettings,
    pub security: SecuritySettings,
    pub users: UsersSettings,
    pub notifications: NotificationsSettings,
    pub services: ServicesSettings,
    pub compositor: CompositorSettings,
    pub terminal: TerminalSettings,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.active = true;
        self.focus_right = false;
        self.selected_category = 0;
    }

    pub fn close(&mut self) {
        self.active = false;
    }

    pub fn toggle(&mut self) {
        if self.active {
            self.close();
        } else {
            self.open();
        }
    }

    /// Handle keyboard input. Returns true if the key was consumed.
    pub fn handle_key(&mut self, key: SettingsKey) -> bool {
        if !self.active {
            return false;
        }
        match key {
            SettingsKey::Escape => {
                self.close();
                true
            }
            SettingsKey::Up => {
                if !self.focus_right && self.selected_category > 0 {
                    self.selected_category -= 1;
                }
                true
            }
            SettingsKey::Down => {
                if !self.focus_right && self.selected_category < CATEGORIES.len() - 1 {
                    self.selected_category += 1;
                }
                true
            }
            SettingsKey::Tab => {
                self.focus_right = !self.focus_right;
                true
            }
            SettingsKey::Enter => {
                // Apply category-specific default action
                true
            }
        }
    }

    /// Return a list of (label, value) lines for the currently selected category.
    pub fn category_content(&self) -> Vec<(&'static str, String)> {
        match self.selected_category {
            0 => vec![
                ("Theme", ["Tron", "Matrix", "Amber"][self.appearance.theme_index].to_string()),
                ("Border Glow", format!("{:.0}%", self.appearance.border_glow * 100.0)),
                ("Font Size", format!("{}pt", self.appearance.font_size)),
            ],
            1 => vec![
                ("Scale", format!("{:.1}x", self.display.scale)),
                ("Refresh Rate", format!("{}Hz", self.display.refresh_rate)),
            ],
            2 => vec![
                ("Keyboard Layout", self.input.kb_layout.clone()),
                ("Mouse Sensitivity", format!("{:.1}", self.input.mouse_sensitivity)),
                ("Scroll Speed", format!("{:.1}", self.input.scroll_speed)),
            ],
            3 => vec![
                ("Volume", format!("{}%", self.audio.volume)),
                ("Muted", self.audio.muted.to_string()),
            ],
            4 => vec![
                ("Wi-Fi", if self.network.wifi_enabled { "Enabled".to_string() } else { "Disabled".to_string() }),
                ("Active Connection", if self.network.active_connection.is_empty() { "None".to_string() } else { self.network.active_connection.clone() }),
            ],
            5 => vec![
                ("Bluetooth", if self.bluetooth.enabled { "Enabled".to_string() } else { "Disabled".to_string() }),
            ],
            6 => vec![
                ("Screen Blank", format!("{}s", self.power.screen_blank_secs)),
                ("Suspend After", format!("{}s", self.power.suspend_secs)),
                ("Hibernate on Low Battery", self.power.hibernate_on_low.to_string()),
            ],
            7 => vec![
                ("Lock Timeout", format!("{}s", self.security.lock_timeout_secs)),
                ("Tailscale VPN", self.security.tailscale_enabled.to_string()),
                ("WireGuard VPN", self.security.wireguard_enabled.to_string()),
                ("Tor", self.security.tor_enabled.to_string()),
            ],
            8 => vec![
                ("User", self.users.display_name.clone()),
            ],
            9 => vec![
                ("Do Not Disturb", self.notifications.do_not_disturb.to_string()),
                ("History Limit", self.notifications.history_limit.to_string()),
            ],
            10 => vec![
                ("Active Services", self.services.service_list.len().to_string()),
            ],
            11 => vec![
                ("Border Width", format!("{}px", self.compositor.border_width)),
                ("Gap Size", format!("{}px", self.compositor.gap_size)),
                ("Animation Speed", format!("{:.1}x", self.compositor.animation_speed)),
            ],
            12 => vec![
                ("Shell", self.terminal.shell.clone()),
                ("Scrollback Lines", self.terminal.scrollback.to_string()),
                ("Cursor Style", ["Block", "Underline", "Bar"][self.terminal.cursor_style].to_string()),
                ("Font Size", format!("{}pt", self.terminal.font_size)),
            ],
            13 => {
                let kernel = std::fs::read_to_string("/proc/version")
                    .unwrap_or_default()
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("unknown")
                    .to_string();
                let hostname = std::fs::read_to_string("/etc/hostname")
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                vec![
                    ("eDEX-DE Version", env!("CARGO_PKG_VERSION").to_string()),
                    ("Hostname", hostname),
                    ("Kernel", kernel),
                    ("License", "GPL-3.0".to_string()),
                    ("Repository", "https://github.com/eDEX-OS/eDEX-DE".to_string()),
                ]
            }
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsKey {
    Up,
    Down,
    Tab,
    Enter,
    Escape,
}
