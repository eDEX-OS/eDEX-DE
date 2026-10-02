//! Handling of `edex-de ipc …` requests.

use ipc::{Request, Response, Target};
use platform::Platform;
use system::SysRequest;
use ui::state::{Osd, OverlayKind, PanelFocus};

use crate::{app::App, events::AppEvent};

fn target_kind(t: &Target) -> OverlayKind {
    match t {
        Target::Launcher => OverlayKind::Launcher,
        Target::Settings => OverlayKind::Settings,
        Target::Privacy => OverlayKind::Privacy,
        Target::Notifications => OverlayKind::Notifications,
        Target::Power => OverlayKind::Power,
    }
}

pub fn drain(app: &mut App, platform: &mut Platform<AppEvent>) {
    let Some(server) = app.ipc.as_ref() else {
        return;
    };
    let pending = server.accept_all();
    for p in pending {
        let request = p.request.clone();
        let response = handle(app, platform, request);
        p.reply(&response);
    }
}

pub fn show_osd(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    label: &str,
    value: f32,
    muted: bool,
) {
    app.state.osd = Some(Osd {
        label: label.into(),
        value,
        muted,
        shown_at: std::time::Instant::now(),
    });
    app.sync_toast_surface(platform);
}

fn handle(app: &mut App, platform: &mut Platform<AppEvent>, req: Request) -> Response {
    match req {
        Request::Ping => Response::with_data(
            serde_json::json!({"version": app.state.version, "uptime_secs": app.started.elapsed().as_secs()}),
        ),
        Request::Toggle { target } => {
            app.toggle_overlay(platform, target_kind(&target));
            Response::ok()
        }
        Request::Show { target } => {
            app.open_overlay(platform, target_kind(&target));
            Response::ok()
        }
        Request::Hide { target } => {
            if app.state.overlay == Some(target_kind(&target)) {
                app.close_overlay(platform);
            }
            Response::ok()
        }
        Request::Focus { target } => {
            match target.as_str() {
                "terminal" => app.state.focus = PanelFocus::Terminal,
                "filesystem" | "files" => app.state.focus = PanelFocus::Filesystem,
                other => return Response::err(format!("unknown focus target {other}")),
            }
            app.focus_shell(platform);
            app.mark_canvas_dirty();
            Response::ok()
        }
        Request::Audio { op, delta, set } => {
            let volume = app.sys.audio.volume as f32;
            match op.as_str() {
                "volume" => {
                    if let Some(v) = set {
                        crate::status::audio_request(app, SysRequest::AudioSetVolume(v.min(150)));
                        show_osd(app, platform, "VOLUME", v.min(150) as f32 / 100.0, false);
                    } else if let Some(d) = delta {
                        crate::status::audio_request(app, SysRequest::AudioAdjustVolume(d));
                        show_osd(
                            app,
                            platform,
                            "VOLUME",
                            ((volume + d as f32) / 100.0).clamp(0.0, 1.5),
                            app.sys.audio.muted,
                        );
                    } else {
                        return Response::err("volume needs delta or set");
                    }
                }
                "mute" => {
                    crate::status::audio_request(app, SysRequest::AudioToggleMute);
                    show_osd(
                        app,
                        platform,
                        "VOLUME",
                        volume / 100.0,
                        !app.sys.audio.muted,
                    );
                }
                "mic-mute" => {
                    crate::status::audio_request(app, SysRequest::AudioToggleMicMute);
                    show_osd(
                        app,
                        platform,
                        "MICROPHONE",
                        app.sys.audio.mic_volume as f32 / 100.0,
                        !app.sys.audio.mic_muted,
                    );
                }
                other => return Response::err(format!("unknown audio op {other}")),
            }
            Response::ok()
        }
        Request::Brightness { delta } => {
            app.system.send(SysRequest::BrightnessAdjust(delta));
            let v = ((app.sys.brightness.percent as i32 + delta).clamp(1, 100)) as f32 / 100.0;
            show_osd(app, platform, "BRIGHTNESS", v, false);
            Response::ok()
        }
        Request::Theme { name } => {
            if app.set_theme(platform, &name) {
                Response::ok()
            } else {
                Response::err(format!(
                    "unknown theme {name}; available: {}",
                    app.themes.keys().cloned().collect::<Vec<_>>().join(", ")
                ))
            }
        }
        Request::Reload => {
            app.reload_config(platform);
            app.export_hypr();
            Response::ok()
        }
        Request::State => Response::with_data(state_json(app, platform)),
        Request::ScreenshotScene => {
            let Some(shell) = app.primary() else {
                return Response::err("no output");
            };
            let (w, h) = shell.size;
            let (rendered, _) = ui::shell::render_canvas(&app.state, w as f32, h as f32);
            let texts: Vec<String> = rendered
                .scene
                .texts
                .iter()
                .map(|t| t.spans.iter().map(|s| s.text.as_str()).collect::<String>())
                .filter(|t| !t.trim().is_empty())
                .collect();
            Response::with_data(
                serde_json::json!({"width": w, "height": h, "rects": rendered.scene.rects.len(), "texts": texts, "hits": rendered.hits.len()}),
            )
        }
        Request::Notify { summary, body } => {
            crate::status::push_local_notification(app, platform, &summary, &body);
            Response::ok()
        }
        Request::Files { path } => {
            let path = path.map(|p| {
                let p = p.strip_prefix("file://").unwrap_or(&p).to_string();
                std::path::PathBuf::from(p)
            });
            match app.open_files(platform, path) {
                Ok(()) => Response::ok(),
                Err(e) => Response::err(e),
            }
        }
        Request::Action { name } => match name.as_str() {
            "install" => {
                let _ = launcher::runner::spawn_detached("edex-install", app.hypr.is_some());
                Response::ok()
            }
            "lock" => {
                let _ = launcher::runner::spawn_detached("hyprlock", app.hypr.is_some());
                Response::ok()
            }
            "new-tab" => {
                let _ = app.terminal.new_tab();
                app.mark_canvas_dirty();
                Response::ok()
            }
            "side-panels" => {
                // Off: apps tile over the file and system panels and get the full width.
                let keep = !app.config.layout.reserve_side_panels;
                app.config.layout.reserve_side_panels = keep;
                app.commit_config(platform, false);
                show_osd(
                    app,
                    platform,
                    if keep {
                        "SIDE PANELS: SHOWN"
                    } else {
                        "SIDE PANELS: HIDDEN"
                    },
                    if keep { 1.0 } else { 0.0 },
                    !keep,
                );
                Response::ok()
            }
            "keyboard" => {
                app.config.appearance.keyboard_visible = !app.config.appearance.keyboard_visible;
                app.commit_config(platform, false);
                Response::ok()
            }
            other => Response::err(format!("unknown action {other}")),
        },
        Request::Quit => {
            app.quit = true;
            Response::ok()
        }
    }
}

pub fn state_json(app: &App, platform: &Platform<AppEvent>) -> serde_json::Value {
    let outputs: Vec<serde_json::Value> = app
        .outputs
        .iter()
        .map(|o| {
            serde_json::json!({
                "name": o.name,
                "size": [o.size.0, o.size.1],
                "canvas_configured": platform.is_configured(o.canvas),
                "reservers": o.reservers.iter().map(|r| platform.is_configured(*r)).collect::<Vec<_>>(),
                "overlay": o.overlay.map(|s| platform.is_configured(s)),
                "toast": o.toast.map(|s| platform.is_configured(s)),
                "scale": platform.scale(o.canvas),
            })
        })
        .collect();
    serde_json::json!({
        "version": app.state.version,
        "uptime_secs": app.started.elapsed().as_secs(),
        "frames": app.frames,
        "gpu": app.gpu.adapter_info(),
        "outputs": outputs,
        "hyprland": {"connected": app.hypr_state.connected, "version": app.hypr_state.version, "workspaces": app.state.workspaces.len(), "active_window": app.state.active_window},
        "overlay": app.state.overlay.map(|k| format!("{k:?}").to_lowercase()),
        "focus": format!("{:?}", app.state.focus).to_lowercase(),
        "terminal": {"tabs": app.terminal.len(), "active": app.terminal.active_index(), "grid": app.terminal.grid_size(), "title": app.state.terminal.frame.title},
        "input": {"keyboard": platform.input_devices().0, "pointer": platform.input_devices().1, "shell_focused": app.state.shell_focused},
        "toasts": app.state.toasts.len(),
        "notifications_received": app.smoke_toasts_seen,
        "notification_server": app.notif_server.as_ref().map(|s| s.is_owner()).unwrap_or(false),
        "theme": app.state.theme.name,
        "live_iso": app.state.live_iso,
        "status": {"volume": app.state.status.volume, "battery": app.state.status.battery_pct, "tor_mode": app.state.status.tor_mode, "wifi": app.state.status.wifi_ssid},
    })
}
