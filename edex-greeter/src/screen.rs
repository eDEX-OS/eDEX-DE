//! Drawing of the greeter screen.

use ui::{
    geometry::{with_alpha, Rect},
    hit::{HitMap, HitTarget},
    layout::Metrics,
    scene::{Align, RectKind, Scanlines, Scene},
    theme::Theme,
    widgets::Ctx,
};

use crate::{Greeter, Phase};

pub const HIT_USER: u32 = 100;
pub const HIT_SESSION_PREV: u32 = 1;
pub const HIT_SESSION_NEXT: u32 = 2;
pub const HIT_INPUT: u32 = 3;
pub const HIT_SUBMIT: u32 = 4;
pub const HIT_SUSPEND: u32 = 10;
pub const HIT_REBOOT: u32 = 11;
pub const HIT_POWEROFF: u32 = 12;

pub struct Rendered {
    pub scene: Scene,
    pub hits: HitMap,
}

fn hex_grid(scene: &mut Scene, theme: &Theme, w: f32, h: f32, pulse: f32) {
    let size = 46.0;
    let dx = size * 1.5;
    let dy = size * 0.866;
    let cols = (w / dx) as i32 + 2;
    let rows = (h / dy) as i32 + 2;
    for r in 0..rows {
        for c in 0..cols {
            let x = c as f32 * dx - size + if r % 2 == 0 { 0.0 } else { dx / 2.0 };
            let y = r as f32 * dy - size;
            let d = ((x - w * 0.5).powi(2) + (y - h * 0.5).powi(2)).sqrt() / (w.max(h) * 0.7);
            let a = (0.16 - d * 0.14).max(0.02) * (0.7 + 0.3 * pulse);
            scene.shape(
                RectKind::Hexagon,
                Rect::new(x, y, size, size),
                [0.0, 0.0, 0.0, 0.0],
                with_alpha(theme.border, a),
                1.0,
            );
        }
    }
}

pub fn render(g: &Greeter, width: f32, height: f32) -> Rendered {
    let theme = &g.theme;
    let metrics: &Metrics = &g.metrics;
    let mut scene = Scene::new(width, height, theme.background);
    let mut hits = HitMap::default();
    hex_grid(&mut scene, theme, width, height, g.pulse());
    let mut ctx = Ctx {
        scene: &mut scene,
        hits: &mut hits,
        theme,
        metrics,
        pulse: g.pulse(),
    };
    let line = ctx.line();
    let font = ctx.font();

    let small = ctx.small();
    let anim = g.cfg.animations;
    let t_secs = g.started_secs();

    // Top bar, as in the shell: badge, host, title, date and clock.
    let bar_h = (line * 2.0).round();
    let bar = Rect::new(0.0, 0.0, width, bar_h);
    ctx.scene.fill(bar, theme.panel_bg);
    ctx.scene.hline(
        0.0,
        bar.bottom() - 1.0,
        width,
        with_alpha(theme.border, 0.6),
    );
    let ty = (bar_h - line) / 2.0;
    let badge = Rect::new(12.0, 4.0, 104.0, bar_h - 8.0);
    ctx.scene.fill(badge, with_alpha(theme.border, 0.15));
    ctx.scene.stroke(badge, theme.border, 1.0);
    ctx.scene.text_bold(
        Rect::new(badge.x, ty, badge.w, line),
        font,
        theme.border,
        Align::Center,
        "eDEX-OS",
    );
    ctx.scene.text_aligned(
        Rect::new(badge.right() + 14.0, ty, width * 0.3, line),
        font,
        theme.text_secondary,
        Align::Left,
        g.hostname.clone(),
    );
    ctx.scene.text_aligned(
        Rect::new(width * 0.3, ty, width * 0.4, line),
        font,
        theme.text_secondary,
        Align::Center,
        "// SECURE LOGIN TERMINAL",
    );
    ctx.scene.text_bold(
        Rect::new(width * 0.6, ty, width * 0.4 - 16.0, line),
        font,
        theme.text_primary,
        Align::Right,
        format!("{}   {}", g.date_short(), g.clock),
    );

    // Big clock above the login panel.
    let clock_size = (font * 3.4).round();
    let clock_y = bar_h + (height * 0.06).max(line);
    ctx.scene.text_bold(
        Rect::new(0.0, clock_y, width, clock_size * 1.3),
        clock_size,
        theme.border,
        Align::Center,
        g.clock.clone(),
    );
    ctx.scene.text_aligned(
        Rect::new(0.0, clock_y + clock_size * 1.3, width, line),
        font,
        theme.text_secondary,
        Align::Center,
        g.date.to_uppercase(),
    );

    // Login panel.
    let pw = (width * 0.42).clamp(420.0, 640.0);
    let ph = line * 17.5;
    let top = clock_y + clock_size * 1.3 + line * 2.0;
    let py = top
        .max((height - ph) / 2.0)
        .min((height - ph - line * 3.0).max(top));
    let panel = Rect::new(((width - pw) / 2.0).round(), py.round(), pw, ph);
    let inner = ctx.frame(panel, Some("AUTHENTICATION"));
    // Scan line sweeping down the panel.
    if anim {
        let phase = (t_secs / 3.2).fract();
        let sy = panel.y + phase * panel.h;
        ctx.scene.fill(
            Rect::new(panel.x + 2.0, sy, panel.w - 4.0, 2.0),
            with_alpha(theme.border, 0.22),
        );
        ctx.scene.fill(
            Rect::new(panel.x + 2.0, sy - 10.0, panel.w - 4.0, 10.0),
            with_alpha(theme.border, 0.04),
        );
    }
    // Side panels on wide screens.
    let side_w = ((width - pw) / 2.0 - 48.0).min(380.0);
    if side_w >= 240.0 {
        let left = Rect::new(24.0, panel.y, side_w, ph);
        draw_system(&mut ctx, left, g);
        let right = Rect::new(width - 24.0 - side_w, panel.y, side_w, ph);
        draw_log(&mut ctx, right, g);
    }
    let mut y = inner.y + line * 0.5;
    // The selected user's monogram in a hexagon, top right of the panel.
    let who = if g.cfg.show_users && !g.users.is_empty() {
        g.users
            .get(g.user_idx)
            .map(|u| u.real_name.clone())
            .unwrap_or_default()
    } else {
        g.username_input.clone()
    };
    if let Some(initial) = who.trim().chars().next() {
        let hs = line * 2.4;
        let hex = Rect::new(inner.right() - hs, panel.y - hs * 0.5, hs, hs);
        ctx.scene
            .shape(RectKind::Hexagon, hex, theme.panel_bg, theme.border, 2.0);
        ctx.scene.text_bold(
            Rect::new(hex.x, hex.y + (hs - line * 1.3) / 2.0, hs, line * 1.3),
            font * 1.3,
            theme.border,
            Align::Center,
            initial.to_uppercase().to_string(),
        );
    }

    // User list or user entry.
    if g.cfg.show_users && !g.users.is_empty() {
        ctx.label_small(Rect::new(inner.x, y, inner.w, line), "USER", theme.text_dim);
        y += line;
        let visible = 4usize;
        let start = g
            .user_idx
            .saturating_sub(visible - 1)
            .min(g.users.len().saturating_sub(visible));
        for (i, u) in g.users.iter().enumerate().skip(start).take(visible) {
            let r = Rect::new(inner.x, y, inner.w, line * 1.6);
            let selected = i == g.user_idx;
            ctx.row(r, selected, HitTarget::OverlayItem(HIT_USER + i as u32));
            ctx.scene.text_bold(
                Rect::new(r.x + 12.0, r.y + line * 0.2, r.w * 0.5, line * 1.2),
                font,
                if selected {
                    theme.text_primary
                } else {
                    theme.text_secondary
                },
                Align::Left,
                u.real_name.clone(),
            );
            ctx.scene.text_aligned(
                Rect::new(
                    r.x + r.w * 0.5,
                    r.y + line * 0.2,
                    r.w * 0.5 - 12.0,
                    line * 1.2,
                ),
                font * 0.9,
                if selected {
                    theme.text_secondary
                } else {
                    theme.text_dim
                },
                Align::Right,
                u.name.clone(),
            );
            y += line * 1.6 + 2.0;
        }
        y += line * 0.4;
    } else {
        ctx.label_small(
            Rect::new(inner.x, y, inner.w, line),
            "USERNAME",
            theme.text_dim,
        );
        y += line;
        let r = Rect::new(inner.x, y, inner.w, line * 1.7);
        ctx.text_input(
            r,
            &g.username_input,
            "username",
            g.phase == Phase::PickUser,
            false,
            HitTarget::OverlayItem(HIT_INPUT),
        );
        y += line * 2.2;
    }

    // Prompt.
    let prompt_label = match g.phase {
        Phase::Prompt => g.prompt.trim().trim_end_matches(':').to_uppercase(),
        Phase::Starting => "STARTING SESSION".into(),
        Phase::Busy => "AUTHENTICATING".into(),
        Phase::PickUser => "PASSWORD".into(),
    };
    ctx.label_small(
        Rect::new(inner.x, y, inner.w, line),
        &prompt_label,
        theme.text_dim,
    );
    y += line;
    let r = Rect::new(inner.x, y, inner.w, line * 1.7);
    let editing = matches!(g.phase, Phase::Prompt)
        || (g.phase == Phase::PickUser && (g.cfg.show_users && !g.users.is_empty()));
    ctx.text_input(
        r,
        &g.input,
        if g.phase == Phase::PickUser {
            "press Enter to start"
        } else {
            ""
        },
        editing,
        g.secret,
        HitTarget::OverlayItem(HIT_INPUT),
    );
    y += line * 2.0;
    if g.caps_lock {
        ctx.scene.text_aligned(
            Rect::new(inner.x, y, inner.w, line),
            font * 0.9,
            theme.warning,
            Align::Left,
            "⚠ CAPS LOCK IS ON",
        );
    }
    y += line * 1.1;

    // Message line.
    if let Some((msg, err)) = &g.message {
        ctx.scene.paragraph(
            Rect::new(inner.x, y, inner.w, line * 2.2),
            font * 0.9,
            if *err {
                theme.error
            } else {
                theme.text_secondary
            },
            msg.clone(),
        );
    }
    y += line * 2.4;

    // Session chooser.
    ctx.label_small(
        Rect::new(inner.x, y, inner.w, line),
        "SESSION",
        theme.text_dim,
    );
    y += line;
    let name = g
        .sessions
        .get(g.session_idx)
        .map(|s| {
            if s.x11 {
                format!("{} (X11)", s.name)
            } else {
                s.name.clone()
            }
        })
        .unwrap_or_else(|| g.cfg.fallback_command.clone());
    let bw = line * 1.8;
    ctx.button(
        Rect::new(inner.x, y, bw, line * 1.5),
        "‹",
        HitTarget::OverlayItem(HIT_SESSION_PREV),
        false,
        g.sessions.len() > 1,
    );
    ctx.scene.text_aligned(
        Rect::new(inner.x + bw + 8.0, y, inner.w - 2.0 * bw - 16.0, line * 1.5),
        font,
        theme.text_primary,
        Align::Center,
        name,
    );
    ctx.button(
        Rect::new(inner.right() - bw, y, bw, line * 1.5),
        "›",
        HitTarget::OverlayItem(HIT_SESSION_NEXT),
        false,
        g.sessions.len() > 1,
    );
    y += line * 2.2;

    // Submit.
    ctx.button(
        Rect::new(inner.x, y, inner.w, line * 1.7),
        if g.phase == Phase::Prompt {
            "LOG IN"
        } else {
            "CONTINUE"
        },
        HitTarget::OverlayItem(HIT_SUBMIT),
        true,
        g.phase != Phase::Busy && g.phase != Phase::Starting,
    );

    // Power buttons.
    if g.cfg.power_buttons {
        let labels = [
            ("SUSPEND  F2", HIT_SUSPEND),
            ("REBOOT  F3", HIT_REBOOT),
            ("POWER OFF  F4", HIT_POWEROFF),
        ];
        let w = 150.0;
        let mut x = width - 24.0 - w * 3.0 - 16.0;
        for (label, id) in labels {
            let r = Rect::new(x, height - 24.0 - line * 1.6, w, line * 1.6);
            ctx.button(r, label, HitTarget::OverlayItem(id), false, true);
            x += w + 8.0;
        }
    }
    ctx.scene.text_aligned(
        Rect::new(24.0, height - 24.0 - line * 1.3, width * 0.5, line),
        small,
        theme.text_dim,
        Align::Left,
        format!(
            "edex-greeter {}  ·  ↑↓ user  ·  Tab session  ·  Enter log in",
            env!("CARGO_PKG_VERSION")
        ),
    );

    scene.scanlines = Some(Scanlines {
        color: theme.border,
        intensity: 0.18,
    });
    Rendered { scene, hits }
}

fn fmt_gib(kb: u64) -> String {
    format!("{:.1} GiB", kb as f64 / (1024.0 * 1024.0))
}

fn fmt_uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m")
    } else {
        format!("{h:02}h {m:02}m")
    }
}

/// Live SYSTEM readout (left of the login panel).
fn draw_system(ctx: &mut Ctx, rect: Rect, g: &Greeter) {
    let t = ctx.theme;
    let inner = ctx.frame(rect, Some("SYSTEM"));
    let line = ctx.line();
    let small = ctx.small();
    let s = &g.stats;
    let cw = ctx.metrics.cell_w * 0.9;
    let mut y = inner.y + 4.0;
    let rows: [(&str, String); 5] = [
        ("HOST", g.hostname.clone()),
        ("KERNEL", s.kernel.clone()),
        ("CPU", format!("{} × {}", s.cores, s.cpu_model)),
        ("MEMORY", fmt_gib(s.mem_total_kb)),
        ("UPTIME", fmt_uptime(s.uptime_secs)),
    ];
    let key_w = 70.0;
    for (k, v) in rows {
        if y + line > inner.bottom() {
            return;
        }
        ctx.scene.text_aligned(
            Rect::new(inner.x, y, key_w, line),
            small,
            t.text_dim,
            Align::Left,
            k,
        );
        let v: String = v
            .chars()
            .take(((inner.w - key_w) / cw).max(4.0) as usize)
            .collect();
        ctx.scene.text_aligned(
            Rect::new(inner.x + key_w, y, inner.w - key_w, line),
            small,
            t.text_primary,
            Align::Left,
            v,
        );
        y += line;
    }
    y += line * 0.6;
    // CPU load history.
    let graph_h = (line * 2.6).round();
    if y + line + graph_h > inner.bottom() {
        return;
    }
    ctx.scene.text_aligned(
        Rect::new(inner.x, y, inner.w, line),
        small,
        t.text_secondary,
        Align::Left,
        format!("CPU LOAD  {:.0}%", s.cpu_now() * 100.0),
    );
    y += line;
    let samples: Vec<f32> = {
        let mut v = vec![0.0; 48usize.saturating_sub(s.cpu_history.len())];
        v.extend(s.cpu_history.iter().copied());
        v
    };
    ctx.scene.sparkline(
        Rect::new(inner.x, y, inner.w, graph_h),
        &samples,
        with_alpha(t.border, 0.85),
        with_alpha(t.border, 0.08),
    );
    y += graph_h + line * 0.6;
    if y + line + 8.0 > inner.bottom() {
        return;
    }
    let mem = s.mem_used_kb as f32 / s.mem_total_kb.max(1) as f32;
    ctx.scene.text_aligned(
        Rect::new(inner.x, y, inner.w, line),
        small,
        t.text_secondary,
        Align::Left,
        format!(
            "RAM  {} / {}",
            fmt_gib(s.mem_used_kb),
            fmt_gib(s.mem_total_kb)
        ),
    );
    y += line;
    ctx.meter(Rect::new(inner.x, y, inner.w, 8.0), mem, t.border);
}

/// ACCESS LOG: the greeter's own events, newest at the bottom.
fn draw_log(ctx: &mut Ctx, rect: Rect, g: &Greeter) {
    let t = ctx.theme;
    let inner = ctx.frame(rect, Some("ACCESS LOG"));
    let line = ctx.line();
    let small = ctx.small();
    let cw = ctx.metrics.cell_w * 0.9;
    let rows = ((inner.h - 8.0) / line).floor().max(0.0) as usize;
    let skip = g.log.len().saturating_sub(rows);
    let max_chars = ((inner.w - 80.0) / cw).max(6.0) as usize;
    let mut y = inner.y + 4.0;
    for (i, (ts, text, err)) in g.log.iter().skip(skip).enumerate() {
        let newest = skip + i + 1 == g.log.len();
        ctx.scene.text_aligned(
            Rect::new(inner.x, y, 76.0, line),
            small,
            t.text_dim,
            Align::Left,
            ts.clone(),
        );
        let color = if *err {
            t.error
        } else if newest {
            t.text_primary
        } else {
            t.text_secondary
        };
        let body: String = text.chars().take(max_chars).collect();
        ctx.scene.text_aligned(
            Rect::new(inner.x + 78.0, y, inner.w - 78.0, line),
            small,
            color,
            Align::Left,
            format!("> {body}"),
        );
        y += line;
    }
    // Blinking cursor after the newest entry.
    if g.cfg.animations
        && ((g.started_secs() * 2.0) as u64).is_multiple_of(2)
        && y + line <= inner.bottom()
    {
        ctx.scene
            .fill(Rect::new(inner.x + 78.0, y + 3.0, cw, line - 6.0), t.border);
    }
}
