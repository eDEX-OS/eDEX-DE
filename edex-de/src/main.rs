use std::{
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use renderer::{
    ui::{FsEntry, UiState, TRON},
    wayland_client::LayerShellClient,
    EdexRenderer,
};
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
    if let Some((width, height)) = layer_client.configured_size() {
        renderer.resize(width, height);
    }

    loop {
        layer_client.dispatch_pending()?;
        if let Some((width, height)) = layer_client.configured_size() {
            renderer.resize(width, height);
        }

        let ui_state = sample_ui_state();
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

fn sample_ui_state() -> UiState {
    UiState {
        clock: current_clock_string(),
        hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "edex-host".to_string()),
        theme: &TRON,
        terminal_content: vec![
            "$ smithay compositor: online".to_string(),
            "$ wgpu renderer: phase 2 active".to_string(),
            "$ glyphon text atlas: warmed".to_string(),
        ],
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
