//! Status data flowing into the shell state: sysmon, privacy probes, notifications,
//! and replies from the system backend.

use std::time::Instant;

use notifications::{server::ServerEvent, Notification, Urgency};
use platform::Platform;
use system::{SysReply, SysRequest};
use tracing::{info, warn};
use ui::state::{DiskDisplay, OverlayKind, ProcDisplay, ToastView};

use crate::{app::App, events::AppEvent};

pub fn refresh_sysinfo(app: &mut App) {
    app.sysmon.refresh();
    let s = app.sysmon.snapshot();
    let si = &mut app.state.sysinfo;
    si.cpu_cores = s.cpu_usage.clone();
    si.cpu_model = s.cpu_model.clone();
    si.cpu_freq_mhz = s.cpu_freq_mhz;
    si.cpu_temp_c = s.cpu_temp_c;
    si.load_avg = s.load_avg;
    si.uptime_secs = s.uptime_secs;
    si.kernel = s.kernel.clone();
    si.ram_used_kb = s.ram_used_kb;
    si.ram_total_kb = s.ram_total_kb;
    si.ram_cached_kb = s.ram_cached_kb;
    si.temps = s.temps.clone();
    si.gpus = s
        .gpus
        .iter()
        .map(|g| ui::state::GpuDisplay {
            name: g.name.clone(),
            busy_pct: g.busy_pct,
            vram_used_bytes: g.vram_used_bytes,
            vram_total_bytes: g.vram_total_bytes,
            temp_c: g.temp_c,
            power_w: g.power_w,
            clock_mhz: g.clock_mhz,
        })
        .collect();
    si.swap_used_kb = s.swap_used_kb;
    si.swap_total_kb = s.swap_total_kb;
    si.net_tx_history = s.net_tx_history.clone();
    si.net_rx_history = s.net_rx_history.clone();
    si.net_tx_kbps = s.net_tx_kbps;
    si.net_rx_kbps = s.net_rx_kbps;
    si.net_iface = s.net_iface.clone();
    si.net_ip = s.net_ip.clone();
    si.disks = s
        .disks
        .iter()
        .map(|d| DiskDisplay {
            mount: d.mount.clone(),
            used_pct: if d.total_bytes > 0 {
                d.used_bytes as f32 / d.total_bytes as f32
            } else {
                0.0
            },
            used_str: ui::filesystem::format_size(d.used_bytes),
            total_str: ui::filesystem::format_size(d.total_bytes),
        })
        .collect();
    si.processes = s
        .processes
        .iter()
        .map(|p| ProcDisplay {
            pid: p.pid,
            name: p.name.clone(),
            cpu_pct: p.cpu_pct,
            mem_str: ui::filesystem::format_size(p.mem_kb * 1024),
        })
        .collect();
    app.state.status.battery_pct = s.battery_pct;
    app.state.status.battery_charging = s.battery_charging;
    if !s.hostname.is_empty() {
        app.state.hostname = s.hostname.clone();
    }
}

pub fn refresh_privacy(app: &mut App) {
    let fprintd = app.sysmon.process_running("fprintd");
    let p = app.privacy_probe.probe(fprintd);
    let st = &mut app.state.status;
    st.tor_mode = p.tor_mode.clone();
    st.tor_active = p.tor_active;
    st.tailscale_active = p.tailscale_connected;
    st.vpn_active = p.vpn_active;
    st.wireguard_active = p.wireguard_active;
    st.fprintd_active = p.fprintd_active;
    st.mic_active = p.mic_active;
    st.camera_active = p.camera_active;
}

/// Rebuild the toast list in the shell state from the notification store.
pub fn sync_toasts(app: &mut App) {
    app.state.toasts = app
        .store
        .active
        .iter()
        .map(|n| ToastView {
            id: n.id,
            app: n.app.clone(),
            summary: n.summary.clone(),
            body: n.body.clone(),
            urgency: match n.urgency {
                Urgency::Low => 0,
                Urgency::Normal => 1,
                Urgency::Critical => 2,
            },
            progress: n.progress.unwrap_or(-1.0),
            actions: n.actions.clone(),
        })
        .collect();
    app.state.status.unread_notifications = app.store.unread;
    app.state.status.dnd = app.store.dnd;
    app.mark_canvas_dirty();
    app.mark_toast_dirty();
}

pub fn push_local_notification(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    summary: &str,
    body: &str,
) {
    let id = 0x7f00_0000 + (app.store.history.len() as u32 % 0x00ff_ffff);
    let n = Notification {
        id,
        app: "eDEX-DE".into(),
        icon: String::new(),
        summary: summary.into(),
        body: body.into(),
        actions: Vec::new(),
        urgency: Urgency::Normal,
        timeout_ms: None,
        progress: None,
        transient: false,
        desktop_entry: None,
        created: Instant::now(),
        wall_time: 0,
    };
    app.store.push(n);
    sync_toasts(app);
    app.sync_toast_surface(platform);
}

pub fn notification_event(app: &mut App, platform: &mut Platform<AppEvent>, ev: ServerEvent) {
    match ev {
        ServerEvent::New(n) => {
            info!(app = %n.app, summary = %n.summary, "notification");
            app.store.push(n);
            app.smoke_toasts_seen += 1;
        }
        ServerEvent::CloseRequested(id) => {
            if app.store.dismiss(id) {
                if let Some(s) = &app.notif_server {
                    s.emit_closed(id, notifications::server::CLOSE_REQUESTED);
                }
            }
        }
    }
    sync_toasts(app);
    if app.state.overlay == Some(OverlayKind::Notifications) {
        crate::overlays::refresh_notifications(app);
    }
    app.sync_toast_surface(platform);
}

pub fn system_reply(app: &mut App, platform: &mut Platform<AppEvent>, reply: SysReply) {
    let mut rebuild = true;
    match reply {
        SysReply::Audio(a) => {
            app.state.status.volume = if a.available {
                Some(a.volume as u8)
            } else {
                None
            };
            app.state.status.muted = a.muted;
            app.state.status.mic_muted = a.mic_muted;
            app.sys.audio = a;
            app.mark_canvas_dirty();
        }
        SysReply::Brightness(b) => app.sys.brightness = b,
        SysReply::Network(n) => {
            app.state.status.wifi_ssid = n.active_ssid.clone();
            app.state.status.ethernet = n.ethernet_connected;
            app.sys.network = n;
            app.mark_canvas_dirty();
        }
        SysReply::Bluetooth(b) => {
            app.state.status.bluetooth_on = b.powered;
            app.state.status.bluetooth_connected = b.devices.iter().filter(|d| d.connected).count();
            app.sys.bluetooth = b;
            app.mark_canvas_dirty();
        }
        SysReply::Power(p) => {
            if p.battery_present {
                app.state.status.battery_pct = Some(p.battery_percent.round() as u8);
                app.state.status.battery_charging =
                    p.battery_state == "charging" || p.battery_state == "full";
            }
            app.sys.power = p;
        }
        SysReply::Users(u) => app.sys.users = u,
        SysReply::Services(s) => {
            if s.user {
                app.sys.services_user = s;
            } else {
                app.sys.services_system = s;
            }
        }
        SysReply::Display(d) => app.sys.display = d,
        SysReply::InputLayouts(l) => app.sys.layouts = l,
        SysReply::Privacy(p) => {
            app.state.status.tor_mode = p.tor.mode.clone();
            app.sys.privacy = p;
        }
        SysReply::Fprint(f) => app.sys.fprint = f,
        SysReply::FprintProgress(p) => {
            app.sys.fprint_progress = Some(p);
        }
        SysReply::About(a) => app.sys.about = a,
        SysReply::TailscaleLoginUrl(url) => {
            app.sys.tailscale_login_url = Some(url.clone());
            let _ = launcher::runner::spawn_detached(
                &format!("xdg-open {}", launcher::desktop::shell_quote(&url)),
                true,
            );
            push_local_notification(
                app,
                platform,
                "Tailscale login",
                "The login page was opened in your browser.",
            );
        }
        SysReply::Done { what, ok, message } => {
            if ok {
                app.sys.last_error = None;
                app.state.settings.status = None;
                app.state.privacy.status = None;
                if what == "fprint" {
                    app.sys.fprint_progress = None;
                }
            } else {
                warn!(%what, %message, "system action failed");
                let msg = format!("{what}: {message}");
                app.sys.last_error = Some(msg.clone());
                app.state.settings.status = Some(msg.clone());
                app.state.privacy.status = Some(msg);
                if app.state.overlay.is_none() {
                    push_local_notification(
                        app,
                        platform,
                        "Action failed",
                        &format!("{what}: {message}"),
                    );
                }
            }
            rebuild = true;
        }
    }
    if rebuild {
        match app.state.overlay {
            Some(OverlayKind::Settings) => crate::forms::settings::rebuild(app),
            Some(OverlayKind::Privacy) => crate::forms::privacy::rebuild(app),
            _ => {}
        }
        app.mark_overlay_dirty();
    }
}

/// Request a fresh audio state and show the OSD once it arrives.
pub fn audio_request(app: &mut App, req: SysRequest) {
    app.system.send(req);
    app.system.send(SysRequest::AudioQuery);
}
