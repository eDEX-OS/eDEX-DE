//! Keyboard and pointer routing for the canvas and overlay surfaces.

use std::time::{Duration, Instant};

use platform::{button, Cursor, KeyInput, Platform, SurfaceId, SurfaceRole};
use ui::{
    hit::{HitTarget, ResizeHandle, StatusItem},
    keyboard::{action_for_keysym, find_key, KeyAction, ROWS},
    panels::terminal_view,
    state::{OverlayKind, PanelFocus},
};
use xkbcommon::xkb::keysyms as ks;

use crate::{app::App, events::AppEvent};

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

pub fn term_mods(m: platform::Modifiers) -> terminal::Modifiers {
    terminal::Modifiers {
        ctrl: m.ctrl,
        alt: m.alt,
        shift: m.shift,
        logo: m.logo,
    }
}

// ─── Keyboard ───────────────────────────────────────────────────────────────

pub fn key(app: &mut App, platform: &mut Platform<AppEvent>, surface: SurfaceId, key: KeyInput) {
    // Physical keys light up the on-screen keyboard.
    if let Some(action) = action_for_keysym(key.keysym, key.text.as_deref()) {
        if let Some(pos) = find_key(action) {
            if key.pressed {
                app.state.keyboard.pressed.insert(pos);
            } else {
                app.state.keyboard.pressed.remove(&pos);
            }
            app.mark_canvas_dirty();
        }
    }
    if !key.pressed {
        return;
    }
    if platform.surface_role(surface) == Some(SurfaceRole::Overlay) || app.state.overlay.is_some() {
        crate::overlays::key(app, platform, &key);
        app.mark_overlay_dirty();
        return;
    }
    if !app.state.boot.done {
        app.state.boot.skip();
        app.mark_canvas_dirty();
        return;
    }
    let m = key.modifiers;
    // Global shell shortcuts.
    if m.ctrl && m.shift {
        match key.keysym {
            ks::KEY_T | ks::KEY_t => {
                if let Err(e) = app.terminal.new_tab() {
                    tracing::warn!("new tab: {e:#}");
                }
                app.state.focus = PanelFocus::Terminal;
                app.mark_canvas_dirty();
                return;
            }
            ks::KEY_W | ks::KEY_w => {
                close_tab(app, app.terminal.active_index());
                return;
            }
            ks::KEY_C | ks::KEY_c => {
                if let Some(text) = app.terminal.selection_text() {
                    platform.copy_to_clipboard(text);
                }
                return;
            }
            ks::KEY_V | ks::KEY_v => {
                platform.request_paste();
                return;
            }
            ks::KEY_F | ks::KEY_f => {
                app.state.focus = match app.state.focus {
                    PanelFocus::Terminal => PanelFocus::Filesystem,
                    PanelFocus::Filesystem => PanelFocus::Terminal,
                };
                app.mark_canvas_dirty();
                return;
            }
            ks::KEY_K | ks::KEY_k => {
                app.config.appearance.keyboard_visible = !app.config.appearance.keyboard_visible;
                app.commit_config(platform, false);
                return;
            }
            ks::KEY_Tab | ks::KEY_ISO_Left_Tab => {
                app.terminal.prev_tab();
                app.mark_canvas_dirty();
                return;
            }
            _ => {}
        }
    }
    if m.ctrl && key.keysym == ks::KEY_Tab {
        app.terminal.next_tab();
        app.mark_canvas_dirty();
        return;
    }
    if m.alt && !m.ctrl && (ks::KEY_1..=ks::KEY_9).contains(&key.keysym) {
        let idx = (key.keysym - ks::KEY_1) as usize;
        if idx < app.terminal.len() {
            app.terminal.switch(idx);
            app.mark_canvas_dirty();
        }
        return;
    }
    if m.shift && !m.ctrl {
        match key.keysym {
            ks::KEY_Page_Up => {
                app.terminal.scroll(app.terminal.grid_size().1 as i32 - 1);
                app.mark_canvas_dirty();
                return;
            }
            ks::KEY_Page_Down => {
                app.terminal
                    .scroll(-(app.terminal.grid_size().1 as i32 - 1));
                app.mark_canvas_dirty();
                return;
            }
            _ => {}
        }
    }
    match app.state.focus {
        PanelFocus::Filesystem => filesystem_key(app, &key),
        PanelFocus::Terminal => {
            if app.terminal.active_exited()
                && matches!(key.keysym, ks::KEY_Return | ks::KEY_KP_Enter)
            {
                close_tab(app, app.terminal.active_index());
                return;
            }
            if app
                .terminal
                .key_press(key.keysym, key.text.as_deref(), term_mods(m))
            {
                app.mark_canvas_dirty();
            }
        }
    }
}

fn filesystem_key(app: &mut App, key: &KeyInput) {
    let fs = &mut app.state.filesystem;
    match key.keysym {
        ks::KEY_Up | ks::KEY_k => fs.navigate_up(),
        ks::KEY_Down | ks::KEY_j => fs.navigate_down(),
        ks::KEY_Page_Up => fs.page(-1),
        ks::KEY_Page_Down => fs.page(1),
        ks::KEY_Return | ks::KEY_KP_Enter | ks::KEY_l => {
            let selected = fs.selected;
            if let Some(e) = fs.entries.get(selected) {
                if e.is_dir {
                    fs.enter_selected();
                } else {
                    ui::filesystem::open_external(&e.path);
                }
            }
        }
        ks::KEY_BackSpace | ks::KEY_Left | ks::KEY_h => fs.go_parent(),
        ks::KEY_period => fs.toggle_dotfiles(),
        ks::KEY_F5 | ks::KEY_r => fs.refresh(),
        ks::KEY_Home | ks::KEY_asciitilde => {
            if let Some(home) = std::env::var_os("HOME") {
                *fs = ui::filesystem::FilesystemPanel::at(home.into());
            }
        }
        ks::KEY_t => {
            let cwd = fs.cwd.clone();
            open_terminal_at(app, cwd);
            return;
        }
        ks::KEY_Escape | ks::KEY_Tab => {
            app.state.focus = PanelFocus::Terminal;
        }
        _ => return,
    }
    app.mark_canvas_dirty();
}

fn open_terminal_at(app: &mut App, cwd: std::path::PathBuf) {
    let mut cfg = crate::app_terminal_config(app);
    cfg.working_directory = Some(cwd);
    app.terminal.set_config(cfg);
    if let Err(e) = app.terminal.new_tab() {
        tracing::warn!("new tab: {e:#}");
    }
    app.terminal.set_config(crate::app_terminal_config(app));
    app.state.focus = PanelFocus::Terminal;
    app.mark_canvas_dirty();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    Focus,
    Minimize,
    Restore,
    ToggleMaximize,
    Close,
}

/// Apply a window control through Hyprland; the resulting events refresh the tab strip.
pub fn window_action(app: &mut App, address: &str, action: WindowAction) {
    let Some(h) = &app.hypr else {
        return;
    };
    let result = match action {
        WindowAction::Focus => h.focus_window(address),
        WindowAction::Minimize => h.minimize_window(address),
        WindowAction::Restore => match app.hypr_state.active_workspace_of(None) {
            Some(ws) => h.restore_window(address, ws),
            None => Ok(()),
        },
        WindowAction::ToggleMaximize => h
            .focus_window(address)
            .and_then(|_| h.toggle_maximized(address)),
        WindowAction::Close => h.close_window(address),
    };
    if let Err(e) = result {
        tracing::warn!("window {action:?} {address}: {e:#}");
    }
}

pub fn close_tab(app: &mut App, index: usize) {
    if app.terminal.len() <= 1 {
        // Closing the last tab replaces it with a fresh shell.
        let _ = app.terminal.close_tab(index);
        if let Err(e) = app.terminal.new_tab() {
            tracing::warn!("new tab: {e:#}");
        }
    } else if let Err(e) = app.terminal.close_tab(index) {
        tracing::warn!("close tab: {e:#}");
    }
    app.mark_canvas_dirty();
}

/// Inject an on-screen keyboard press into the terminal.
fn onscreen_key(app: &mut App, platform: &mut Platform<AppEvent>, row: usize, col: usize) {
    let Some(def) = ROWS.get(row).and_then(|r| r.get(col)) else {
        return;
    };
    let kb = &mut app.state.keyboard;
    match def.action {
        KeyAction::Shift => {
            kb.sticky_shift = !kb.sticky_shift;
            return;
        }
        KeyAction::Ctrl => {
            kb.sticky_ctrl = !kb.sticky_ctrl;
            return;
        }
        KeyAction::Alt => {
            kb.sticky_alt = !kb.sticky_alt;
            return;
        }
        KeyAction::Super => return,
        KeyAction::CapsLock => {
            kb.caps_lock = !kb.caps_lock;
            return;
        }
        _ => {}
    }
    let shift = kb.sticky_shift || kb.shift;
    let mods = terminal::Modifiers {
        ctrl: kb.sticky_ctrl || kb.ctrl,
        alt: kb.sticky_alt || kb.alt,
        shift,
        logo: false,
    };
    let (keysym, text) = match def.action {
        KeyAction::Char(c) => {
            let ch = if shift ^ (kb.caps_lock && c.is_ascii_alphabetic()) {
                def.shifted.chars().next().unwrap_or(c.to_ascii_uppercase())
            } else {
                c
            };
            (
                xkbcommon::xkb::utf32_to_keysym(ch as u32).raw(),
                Some(ch.to_string()),
            )
        }
        KeyAction::Escape => (ks::KEY_Escape, None),
        KeyAction::Tab => (ks::KEY_Tab, None),
        KeyAction::Backspace => (ks::KEY_BackSpace, None),
        KeyAction::Enter => (ks::KEY_Return, None),
        KeyAction::Space => (ks::KEY_space, Some(" ".to_string())),
        KeyAction::Up => (ks::KEY_Up, None),
        KeyAction::Down => (ks::KEY_Down, None),
        KeyAction::Left => (ks::KEY_Left, None),
        KeyAction::Right => (ks::KEY_Right, None),
        _ => return,
    };
    kb.sticky_shift = false;
    kb.sticky_ctrl = false;
    kb.sticky_alt = false;
    let input = KeyInput {
        keysym,
        raw_code: 0,
        text,
        pressed: true,
        repeat: false,
        modifiers: platform::Modifiers {
            ctrl: mods.ctrl,
            alt: mods.alt,
            shift: mods.shift,
            logo: false,
            caps_lock: kb.caps_lock,
            num_lock: false,
        },
    };
    if app.state.overlay.is_some() {
        crate::overlays::key(app, platform, &input);
        app.mark_overlay_dirty();
    } else if app.state.focus == PanelFocus::Filesystem {
        filesystem_key(app, &input);
    } else {
        app.terminal.key_press(keysym, input.text.as_deref(), mods);
    }
    app.mark_canvas_dirty();
}

// ─── Pointer ────────────────────────────────────────────────────────────────

fn hit(app: &App, surface: SurfaceId, x: f64, y: f64) -> HitTarget {
    app.hits
        .get(&surface)
        .map(|h| h.resolve(x as f32, y as f32))
        .unwrap_or(HitTarget::None)
}

fn terminal_cell(app: &App, surface: SurfaceId, x: f64, y: f64) -> Option<(usize, usize)> {
    let shell = app.outputs.iter().find(|o| o.canvas == surface)?;
    let layout = shell.layout.as_ref()?;
    let m = &app.state.metrics;
    let g = terminal_view::grid_rect(layout.terminal, m.line);
    let (cols, rows) = app.terminal.grid_size();
    let col = (((x as f32 - g.x) / m.cell_w).floor().max(0.0) as usize).min(cols.saturating_sub(1));
    let row = (((y as f32 - g.y) / m.cell_h).floor().max(0.0) as usize).min(rows.saturating_sub(1));
    Some((col, row))
}

pub fn pointer_motion(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    surface: SurfaceId,
    x: f64,
    y: f64,
) {
    let role = platform.surface_role(surface);
    if role != Some(SurfaceRole::Canvas) {
        let t = hit(app, surface, x, y);
        platform.set_cursor(if matches!(t, HitTarget::None | HitTarget::OverlayPanel) {
            Cursor::Default
        } else {
            Cursor::Pointer
        });
        return;
    }
    // Resize drag in progress.
    if let Some(handle) = app.state.resize.dragging {
        let Some(shell) = app.outputs.iter().find(|o| o.canvas == surface) else {
            return;
        };
        let Some(layout) = shell.layout.as_ref() else {
            return;
        };
        let frac = layout.split_from_x(x as f32);
        match handle {
            ResizeHandle::FsTerminal => app.config.layout.fs_split = frac.clamp(0.08, 0.5),
            ResizeHandle::TerminalSysinfo => {
                app.config.layout.sysinfo_split = frac.clamp(0.5, 0.95)
            }
        }
        app.state.layout_cfg.fs_split = app.config.layout.fs_split;
        app.state.layout_cfg.sysinfo_split = app.config.layout.sysinfo_split;
        app.relayout(platform);
        return;
    }
    let target = hit(app, surface, x, y);
    let mut dirty = false;
    let hover_key = if let HitTarget::KeyboardKey(r, c) = target {
        Some((r, c))
    } else {
        None
    };
    if app.state.keyboard.hover != hover_key {
        app.state.keyboard.hover = hover_key;
        dirty = true;
    }
    let hover_handle = if let HitTarget::ResizeHandle(h) = target {
        Some(h)
    } else {
        None
    };
    if app.state.resize.hover != hover_handle {
        app.state.resize.hover = hover_handle;
        dirty = true;
    }
    platform.set_cursor(match target {
        HitTarget::ResizeHandle(_) => Cursor::ColResize,
        HitTarget::TerminalArea => Cursor::Text,
        HitTarget::None | HitTarget::FilesystemArea => Cursor::Default,
        _ => Cursor::Pointer,
    });
    if app.button_held && target == HitTarget::TerminalArea {
        if let Some((col, row)) = terminal_cell(app, surface, x, y) {
            app.terminal
                .mouse_motion(col, row, term_mods(platform.modifiers()), true);
            dirty = true;
        }
    }
    if dirty {
        app.mark_canvas_dirty();
    }
}

pub fn pointer_button(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    surface: SurfaceId,
    btn: u32,
    pressed: bool,
    x: f64,
    y: f64,
) {
    let role = platform.surface_role(surface);
    if !pressed {
        if btn == button::LEFT {
            app.button_held = false;
            if app.state.resize.dragging.take().is_some() {
                app.commit_config(platform, false);
                return;
            }
            if role == Some(SurfaceRole::Canvas) {
                if let Some((col, row)) = terminal_cell(app, surface, x, y) {
                    app.terminal
                        .mouse_release(col, row, btn, term_mods(platform.modifiers()));
                }
            }
        }
        return;
    }
    let target = hit(app, surface, x, y);
    match role {
        Some(SurfaceRole::Overlay) => {
            crate::overlays::click(app, platform, target, btn, x, y);
            app.mark_overlay_dirty();
        }
        Some(SurfaceRole::Toast) => toast_click(app, platform, target),
        Some(SurfaceRole::Canvas) | Some(SurfaceRole::Strip) => {
            canvas_click(app, platform, surface, target, btn, x, y)
        }
        _ => {}
    }
}

fn toast_click(app: &mut App, platform: &mut Platform<AppEvent>, target: HitTarget) {
    match target {
        HitTarget::Toast(id) => {
            if app.store.dismiss(id) {
                if let Some(s) = &app.notif_server {
                    s.emit_closed(id, notifications::server::CLOSE_DISMISSED);
                }
            }
        }
        HitTarget::ToastAction(code) => {
            let (id, idx) = (code / 8, (code % 8) as usize);
            if let Some(key) = app.store.active_action(id, idx) {
                if let Some(s) = &app.notif_server {
                    s.emit_action(id, &key);
                    s.emit_closed(id, notifications::server::CLOSE_DISMISSED);
                }
                app.store.dismiss(id);
            }
        }
        _ => return,
    }
    crate::status::sync_toasts(app);
    app.sync_toast_surface(platform);
}

fn canvas_click(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    surface: SurfaceId,
    target: HitTarget,
    btn: u32,
    x: f64,
    y: f64,
) {
    if !app.state.boot.done {
        app.state.boot.skip();
        app.mark_canvas_dirty();
        return;
    }
    if app.state.overlay.is_some() {
        app.close_overlay(platform);
    }
    let now = Instant::now();
    let click_count = match app.last_click {
        Some((t, lx, ly, n))
            if now.duration_since(t) < DOUBLE_CLICK
                && (lx - x).abs() < 4.0
                && (ly - y).abs() < 4.0 =>
        {
            (n + 1).min(3)
        }
        _ => 1,
    };
    app.last_click = Some((now, x, y, click_count));
    match target {
        HitTarget::TerminalArea => {
            app.state.focus = PanelFocus::Terminal;
            if btn == button::LEFT {
                app.button_held = true;
            }
            if btn == button::MIDDLE {
                platform.request_paste();
            } else if let Some((col, row)) = terminal_cell(app, surface, x, y) {
                app.terminal.mouse_press(
                    col,
                    row,
                    btn,
                    term_mods(platform.modifiers()),
                    click_count,
                );
            }
        }
        HitTarget::TerminalTab(i) => {
            if btn == button::MIDDLE {
                close_tab(app, i);
            } else {
                app.terminal.switch(i);
                app.state.focus = PanelFocus::Terminal;
                app.bring_terminal_forward(platform);
            }
        }
        HitTarget::TerminalTabClose(i) => close_tab(app, i),
        HitTarget::TerminalNewTab => {
            if let Err(e) = app.terminal.new_tab() {
                tracing::warn!("new tab: {e:#}");
            }
            app.state.focus = PanelFocus::Terminal;
            app.bring_terminal_forward(platform);
        }
        HitTarget::AppTab(i) => {
            if let Some(win) = app.state.windows.get(i).cloned() {
                if btn == button::MIDDLE {
                    window_action(app, &win.address, WindowAction::Close);
                } else if win.minimized {
                    window_action(app, &win.address, WindowAction::Restore);
                } else {
                    window_action(app, &win.address, WindowAction::Focus);
                }
            }
        }
        HitTarget::AppTabClose(i) => {
            if let Some(win) = app.state.windows.get(i).cloned() {
                window_action(app, &win.address, WindowAction::Close);
            }
        }
        HitTarget::WindowMinimize | HitTarget::WindowMaximize | HitTarget::WindowClose => {
            let action = match target {
                HitTarget::WindowMinimize => WindowAction::Minimize,
                HitTarget::WindowMaximize => WindowAction::ToggleMaximize,
                _ => WindowAction::Close,
            };
            if let Some(win) = app.state.windows.iter().find(|w| w.active && !w.minimized) {
                let address = win.address.clone();
                window_action(app, &address, action);
            }
        }
        HitTarget::FilesystemArea => app.state.focus = PanelFocus::Filesystem,
        HitTarget::FsRanger => {
            if let Err(e) = app.open_files(platform, None) {
                crate::status::push_local_notification(app, platform, "Files", &e);
            }
        }
        HitTarget::FsEntry(i) => {
            app.state.focus = PanelFocus::Filesystem;
            let fs = &mut app.state.filesystem;
            fs.select_visible(i);
            if btn == button::RIGHT {
                let cwd = fs
                    .entries
                    .get(fs.selected)
                    .filter(|e| e.is_dir)
                    .map(|e| e.path.clone())
                    .unwrap_or_else(|| fs.cwd.clone());
                open_terminal_at(app, cwd);
                return;
            }
            if click_count >= 2 || btn == button::MIDDLE {
                let selected = fs.selected;
                if let Some(e) = fs.entries.get(selected) {
                    if e.is_dir {
                        fs.enter_selected();
                    } else {
                        ui::filesystem::open_external(&e.path);
                    }
                }
            }
        }
        HitTarget::FsBreadcrumb(i) => {
            app.state.focus = PanelFocus::Filesystem;
            app.state.filesystem.go_breadcrumb(i);
        }
        HitTarget::FsParent => app.state.filesystem.go_parent(),
        HitTarget::KeyboardKey(r, c) => onscreen_key(app, platform, r, c),
        HitTarget::ResizeHandle(h) => {
            if btn == button::LEFT {
                app.state.resize.dragging = Some(h);
            }
        }
        HitTarget::Workspace(id) => {
            if let Some(h) = &app.hypr {
                if let Err(e) = h.focus_workspace(id as i32) {
                    tracing::warn!("workspace switch: {e:#}");
                }
            }
        }
        HitTarget::Install => {
            let _ = launcher::runner::spawn_detached("edex-install", true);
        }
        HitTarget::Launcher => app.open_overlay(platform, OverlayKind::Launcher),
        HitTarget::Clock => app.open_overlay(platform, OverlayKind::Power),
        HitTarget::Status(item) => status_click(app, platform, item, btn),
        HitTarget::None
        | HitTarget::OverlayItem(_)
        | HitTarget::OverlayClose
        | HitTarget::OverlayPanel
        | HitTarget::Toast(_)
        | HitTarget::ToastAction(_) => {}
    }
    app.mark_canvas_dirty();
}

fn status_click(app: &mut App, platform: &mut Platform<AppEvent>, item: StatusItem, btn: u32) {
    match item {
        StatusItem::Volume => {
            if btn == button::RIGHT || btn == button::MIDDLE {
                crate::status::audio_request(app, system::SysRequest::AudioToggleMute);
            } else {
                app.open_overlay(platform, OverlayKind::Settings);
                crate::forms::settings::select_tab(app, crate::forms::settings::TAB_AUDIO);
            }
        }
        StatusItem::Battery => {
            app.open_overlay(platform, OverlayKind::Settings);
            crate::forms::settings::select_tab(app, crate::forms::settings::TAB_POWER);
        }
        StatusItem::Network => {
            app.open_overlay(platform, OverlayKind::Settings);
            crate::forms::settings::select_tab(app, crate::forms::settings::TAB_NETWORK);
        }
        StatusItem::Notifications => {
            if btn == button::RIGHT {
                app.store.dnd = !app.store.dnd;
                app.config.notifications.dnd = app.store.dnd;
                app.commit_config(platform, false);
                crate::status::sync_toasts(app);
            } else {
                app.open_overlay(platform, OverlayKind::Notifications);
            }
        }
        StatusItem::Tor
        | StatusItem::Tailscale
        | StatusItem::Vpn
        | StatusItem::WireGuard
        | StatusItem::Fingerprint
        | StatusItem::Microphone
        | StatusItem::Camera => {
            app.open_overlay(platform, OverlayKind::Privacy);
            let tab = match item {
                StatusItem::Tailscale => crate::forms::privacy::TAB_TAILSCALE,
                StatusItem::Vpn | StatusItem::WireGuard => crate::forms::privacy::TAB_VPN,
                StatusItem::Fingerprint | StatusItem::Microphone | StatusItem::Camera => {
                    crate::forms::privacy::TAB_DEVICES
                }
                _ => crate::forms::privacy::TAB_TOR,
            };
            crate::forms::privacy::select_tab(app, tab);
        }
    }
}

pub fn pointer_axis(
    app: &mut App,
    platform: &mut Platform<AppEvent>,
    surface: SurfaceId,
    _dx: f64,
    dy: f64,
    x: f64,
    y: f64,
) {
    let lines = if dy.abs() < 1.0 {
        0
    } else {
        (dy / 20.0).round() as i32
    }
    .clamp(-10, 10);
    if lines == 0 {
        return;
    }
    let target = hit(app, surface, x, y);
    match platform.surface_role(surface) {
        Some(SurfaceRole::Overlay) => {
            crate::overlays::scroll(app, lines);
            app.mark_overlay_dirty();
        }
        Some(SurfaceRole::Canvas) => {
            match target {
                HitTarget::TerminalArea => {
                    if let Some((col, row)) = terminal_cell(app, surface, x, y) {
                        app.terminal.mouse_scroll(
                            col,
                            row,
                            -lines,
                            term_mods(platform.modifiers()),
                        );
                    }
                }
                HitTarget::FilesystemArea | HitTarget::FsEntry(_) => {
                    app.state.filesystem.scroll_by(lines)
                }
                HitTarget::Status(StatusItem::Volume) => {
                    crate::status::audio_request(
                        app,
                        system::SysRequest::AudioAdjustVolume(if lines > 0 { -5 } else { 5 }),
                    );
                }
                _ => return,
            }
            app.mark_canvas_dirty();
        }
        _ => {}
    }
}
