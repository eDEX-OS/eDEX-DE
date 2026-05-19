use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use renderer::{
    ui::{
        builtin_theme, BootAnimation, DiskDisplay, FilesystemPanel, PanelLayout, ProcDisplay,
        ResizeState, StatusInfo, SysInfo, UiState,
    },
    wayland_client::{KeyEvent, LayerShellClient},
    EdexRenderer,
};
use sysmon::{SysSnapshot, SysmonCollector};
use terminal::{key_event_to_bytes, Clipboard, Modifiers, TerminalTabs};
use tracing::info;
use xkbcommon::xkb::keysyms::{
    KEY_BackSpace as KEY_BACK_SPACE, KEY_Down as KEY_DOWN, KEY_KP_Enter as KEY_KP_ENTER,
    KEY_Return as KEY_RETURN, KEY_Up as KEY_UP,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("eDEX-DE v{}", env!("CARGO_PKG_VERSION"));
    info!("Initializing compositor + renderer threads...");

    let (socket_tx, socket_rx) = mpsc::channel();
    let compositor_thread = thread::Builder::new()
        .name("edex-compositor".to_string())
        .spawn(move || {
            compositor::run_compositor_with_socket_notifier(move |socket_name| {
                let _ = socket_tx.send(socket_name);
            })
        })
        .context("failed to spawn compositor thread")?;

    let socket_name = socket_rx
        .recv_timeout(EdexRenderer::frame_interval() * 300)
        .context("timed out waiting for compositor Wayland socket")?;
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    info!(socket = ?socket_name, "renderer connecting to compositor socket");

    let mut layer_client = LayerShellClient::connect_from_env()?;
    for _ in 0..60 {
        layer_client.roundtrip()?;
        if layer_client.configured_size().is_some() {
            break;
        }
        thread::sleep(EdexRenderer::frame_interval());
    }

    let mut renderer = EdexRenderer::new(layer_client.connection(), layer_client.surface())?;
    let mut surface_size = layer_client.configured_size().unwrap_or((1280, 720));
    renderer.resize(surface_size.0, surface_size.1);

    let (cols, rows) = terminal_dimensions(surface_size.0, surface_size.1);
    let mut tabs = TerminalTabs::new(cols, rows)?;
    let clipboard = Clipboard::default();
    let hostname = read_hostname();
    let theme = builtin_theme("tron")?;
    let mut boot_anim = BootAnimation::new();
    let mut boot_done = false;
    let mut border_anim = 0.0_f32;
    let mut filesystem = FilesystemPanel::new();
    filesystem.set_panel_height(
        PanelLayout::from_size(surface_size.0, surface_size.1)
            .filesystem
            .height,
    );
    let mut collector = SysmonCollector::new();
    let mut sys_snapshot = collector.snapshot();
    let mut last_sys_refresh = Instant::now();

    loop {
        layer_client.dispatch_pending()?;
        if let Some((width, height)) = layer_client.configured_size() {
            if (width, height) != surface_size {
                surface_size = (width, height);
                renderer.resize(width, height);
                let (cols, rows) = terminal_dimensions(width, height);
                tabs.resize_all(cols, rows)?;
            }
        }

        let layout = PanelLayout::from_size(surface_size.0, surface_size.1);
        filesystem.set_panel_height(layout.filesystem.height);

        for event in layer_client.drain_key_events() {
            handle_key_event(&mut tabs, &clipboard, &mut filesystem, event, surface_size)?;
        }

        if last_sys_refresh.elapsed() >= Duration::from_secs(1) {
            collector.refresh();
            sys_snapshot = collector.snapshot();
            last_sys_refresh = Instant::now();
        }

        if !boot_done {
            boot_done = boot_anim.update();
        }
        border_anim = (border_anim + 0.016) % std::f32::consts::TAU;

        let ui_state = build_ui_state(UiStateInput {
            theme,
            hostname: &hostname,
            terminal_content: tabs.visible_lines(),
            tab_count: tabs.len(),
            active_tab: tabs.active_index(),
            filesystem: &filesystem,
            sys_snapshot: &sys_snapshot,
            boot_done,
            boot_anim: &boot_anim,
            border_anim,
        });
        renderer.render(&ui_state)?;
        thread::sleep(EdexRenderer::frame_interval());

        if compositor_thread.is_finished() {
            compositor_thread
                .join()
                .map_err(|_| anyhow!("compositor thread panicked"))??;
            break;
        }
    }

    Ok(())
}

struct UiStateInput<'a> {
    theme: &'static renderer::ui::Theme,
    hostname: &'a str,
    terminal_content: Vec<String>,
    tab_count: usize,
    active_tab: usize,
    filesystem: &'a FilesystemPanel,
    sys_snapshot: &'a SysSnapshot,
    boot_done: bool,
    boot_anim: &'a BootAnimation,
    border_anim: f32,
}

fn build_ui_state(input: UiStateInput<'_>) -> UiState {
    UiState {
        clock: current_clock_string(),
        date: current_date_string(),
        hostname: input.hostname.to_string(),
        theme: input.theme,
        terminal_content: input.terminal_content,
        filesystem_cwd: input.filesystem.breadcrumbs().join(" / "),
        filesystem_entries: input.filesystem.to_ui_entries(),
        selected_fs_entry: input.filesystem.selected_visible_index(),
        sysinfo: build_sysinfo(input.sys_snapshot),
        boot_done: input.boot_done,
        boot_lines: if input.boot_done {
            Vec::new()
        } else {
            input.boot_anim.current_lines().to_vec()
        },
        boot_overlay_alpha: input.boot_anim.overlay_alpha(),
        border_anim: input.border_anim,
        resize: ResizeState::default(),
        tab_count: input.tab_count,
        active_tab: input.active_tab,
        status: build_status(input.sys_snapshot),
    }
}

fn build_status(snapshot: &SysSnapshot) -> StatusInfo {
    StatusInfo {
        volume: 42,
        battery_pct: snapshot.battery_pct,
        battery_charging: snapshot.battery_charging,
        tor_active: false,
        tailscale_active: false,
        vpn_active: false,
        net_tx_kbps: snapshot.net_tx_kbps,
        net_rx_kbps: snapshot.net_rx_kbps,
    }
}

fn build_sysinfo(snapshot: &SysSnapshot) -> SysInfo {
    SysInfo {
        cpu_cores: snapshot.cpu_usage.clone(),
        cpu_model: snapshot.cpu_model.clone(),
        ram_used_kb: snapshot.ram_used_kb,
        ram_total_kb: snapshot.ram_total_kb,
        swap_used_kb: snapshot.swap_used_kb,
        swap_total_kb: snapshot.swap_total_kb,
        net_tx_history: snapshot.net_tx_history.clone(),
        net_rx_history: snapshot.net_rx_history.clone(),
        disks: snapshot
            .disks
            .iter()
            .map(|disk| {
                let used_pct = if disk.total_bytes == 0 {
                    0.0
                } else {
                    disk.used_bytes as f32 / disk.total_bytes as f32 * 100.0
                };
                DiskDisplay {
                    mount: disk.mount.clone(),
                    used_pct,
                    used_str: format_bytes(disk.used_bytes),
                    total_str: format_bytes(disk.total_bytes),
                }
            })
            .collect(),
        processes: snapshot
            .processes
            .iter()
            .map(|proc| ProcDisplay {
                pid: proc.pid,
                name: proc.name.clone(),
                cpu_pct: proc.cpu_pct,
                mem_str: format_kib(proc.mem_kb),
            })
            .collect(),
    }
}

fn handle_key_event(
    tabs: &mut TerminalTabs,
    clipboard: &Clipboard,
    filesystem: &mut FilesystemPanel,
    event: KeyEvent,
    surface_size: (u32, u32),
) -> Result<()> {
    if !event.pressed {
        return Ok(());
    }

    let modifiers = Modifiers {
        ctrl: event.modifiers.ctrl,
        alt: event.modifiers.alt,
        shift: event.modifiers.shift,
        logo: event.modifiers.logo,
    };

    if modifiers.ctrl && modifiers.shift && shortcut_is(event.keysym, 't') {
        let (cols, rows) = terminal_dimensions(surface_size.0, surface_size.1);
        tabs.new_tab(cols, rows)?;
        return Ok(());
    }

    if modifiers.ctrl && modifiers.shift && shortcut_is(event.keysym, 'w') {
        let active = tabs.active_index();
        tabs.close_tab(active);
        return Ok(());
    }

    if modifiers.ctrl && modifiers.shift && shortcut_is(event.keysym, 'c') {
        clipboard.copy(&tabs.visible_lines().join("\n"));
        return Ok(());
    }

    if modifiers.ctrl && modifiers.shift && shortcut_is(event.keysym, 'v') {
        if let Some(text) = clipboard.paste() {
            tabs.write_input(text.as_bytes())?;
        }
        return Ok(());
    }

    if modifiers.ctrl && modifiers.shift && shortcut_is(event.keysym, 'h') {
        filesystem.toggle_dotfiles();
        return Ok(());
    }

    if !modifiers.ctrl && !modifiers.alt && !modifiers.logo {
        match event.keysym {
            KEY_UP => {
                filesystem.navigate_up();
                return Ok(());
            }
            KEY_DOWN => {
                filesystem.navigate_down();
                return Ok(());
            }
            KEY_RETURN | KEY_KP_ENTER => {
                filesystem.enter_selected();
                return Ok(());
            }
            KEY_BACK_SPACE => {
                filesystem.go_parent();
                return Ok(());
            }
            _ => {}
        }
    }

    if modifiers.alt && !modifiers.ctrl && !modifiers.logo {
        if let Some(index) = shortcut_tab_index(event.keysym) {
            tabs.switch(index);
            return Ok(());
        }
    }

    if let Some(bytes) = key_event_to_bytes(event.keysym, &event.text, modifiers) {
        tabs.write_input(&bytes)?;
    }

    Ok(())
}

fn terminal_dimensions(width: u32, height: u32) -> (usize, usize) {
    const CELL_WIDTH: u32 = 10;
    const CELL_HEIGHT: u32 = 20;
    let terminal_height = height.saturating_sub((height / 5).max(120));
    let cols = (width / CELL_WIDTH).max(40) as usize;
    let rows = (terminal_height / CELL_HEIGHT).max(16) as usize;
    (cols, rows)
}

fn shortcut_is(keysym: u32, expected: char) -> bool {
    char::from_u32(keysym)
        .map(|ch| ch.to_ascii_lowercase() == expected)
        .unwrap_or(false)
}

fn shortcut_tab_index(keysym: u32) -> Option<usize> {
    char::from_u32(keysym)
        .and_then(|ch| ch.to_digit(10))
        .and_then(|digit| digit.checked_sub(1))
        .map(|digit| digit as usize)
        .filter(|index| *index < 9)
}

fn current_clock_string() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn current_date_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn read_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|content| content.trim().to_string())
        .ok()
        .filter(|hostname| !hostname.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "edex-host".to_string())
}

fn format_kib(kib: u64) -> String {
    const KIB_PER_MIB: u64 = 1024;
    const KIB_PER_GIB: u64 = 1024 * 1024;

    if kib >= KIB_PER_GIB {
        format!("{:.1} GiB", kib as f64 / KIB_PER_GIB as f64)
    } else if kib >= KIB_PER_MIB {
        format!("{:.1} MiB", kib as f64 / KIB_PER_MIB as f64)
    } else {
        format!("{} KiB", kib)
    }
}

fn format_bytes(bytes: u64) -> String {
    const BYTES_PER_KIB: u64 = 1024;
    const BYTES_PER_MIB: u64 = 1024 * 1024;
    const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;

    if bytes >= BYTES_PER_GIB {
        format!("{:.1} GiB", bytes as f64 / BYTES_PER_GIB as f64)
    } else if bytes >= BYTES_PER_MIB {
        format!("{:.1} MiB", bytes as f64 / BYTES_PER_MIB as f64)
    } else if bytes >= BYTES_PER_KIB {
        format!("{:.1} KiB", bytes as f64 / BYTES_PER_KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}
