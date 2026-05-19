use std::{
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use renderer::{
    ui::{FsEntry, UiState, TRON},
    wayland_client::{KeyEvent, LayerShellClient},
    EdexRenderer,
};
use terminal::{key_event_to_bytes, Clipboard, Modifiers, TerminalTabs};
use tracing::info;

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
        .recv_timeout(Duration::from_secs(5))
        .context("timed out waiting for compositor Wayland socket")?;
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    info!(socket = ?socket_name, "renderer connecting to compositor socket");

    let mut layer_client = LayerShellClient::connect_from_env()?;
    for _ in 0..60 {
        layer_client.roundtrip()?;
        if layer_client.configured_size().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(16));
    }

    let mut renderer = EdexRenderer::new(layer_client.connection(), layer_client.surface())?;
    let mut surface_size = layer_client.configured_size().unwrap_or((1280, 720));
    renderer.resize(surface_size.0, surface_size.1);

    let (cols, rows) = terminal_dimensions(surface_size.0, surface_size.1);
    let mut tabs = TerminalTabs::new(cols, rows)?;
    let clipboard = Clipboard::default();

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

        for event in layer_client.drain_key_events() {
            handle_key_event(&mut tabs, &clipboard, event, surface_size)?;
        }

        let ui_state = build_ui_state(tabs.visible_lines());
        renderer.render(&ui_state)?;
        thread::sleep(EdexRenderer::frame_interval());

        if compositor_thread.is_finished() {
            compositor_thread
                .join()
                .map_err(|_| anyhow::anyhow!("compositor thread panicked"))??;
            break;
        }
    }

    Ok(())
}

fn build_ui_state(terminal_content: Vec<String>) -> UiState {
    UiState {
        clock: current_clock_string(),
        hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "edex-host".to_string()),
        theme: &TRON,
        terminal_content,
        filesystem_cwd: "/home/aric/edex-ui-hyprland".to_string(),
        filesystem_entries: vec![
            FsEntry {
                name: "compositor/".to_string(),
                is_dir: true,
            },
            FsEntry {
                name: "renderer/".to_string(),
                is_dir: true,
            },
            FsEntry {
                name: "terminal/".to_string(),
                is_dir: true,
            },
            FsEntry {
                name: "edex-de/".to_string(),
                is_dir: true,
            },
            FsEntry {
                name: "Cargo.toml".to_string(),
                is_dir: false,
            },
        ],
    }
}

fn handle_key_event(
    tabs: &mut TerminalTabs,
    clipboard: &Clipboard,
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
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_seconds = elapsed % 86_400;
    let hours = day_seconds / 3_600;
    let minutes = (day_seconds % 3_600) / 60;
    let seconds = day_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
