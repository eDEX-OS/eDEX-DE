//! System information dashboard.

use crate::{
    geometry::{with_alpha, Rect},
    scene::Align,
    state::ShellState,
    widgets::Ctx,
};

/// Draw a section header and return the body rect of `height`; advances `rect` past it.
fn section(ctx: &mut Ctx, rect: &mut Rect, title: &str, height: f32) -> Option<Rect> {
    let line = ctx.line();
    if height + line + 6.0 > rect.h {
        return None;
    }
    let t = ctx.theme;
    let head = Rect::new(rect.x, rect.y, rect.w, line);
    let s = ctx.small();
    ctx.scene.text_bold(head, s, t.border, Align::Left, title);
    ctx.scene.hline(
        rect.x,
        head.bottom() - 2.0,
        rect.w,
        with_alpha(t.border, 0.35),
    );
    let body = Rect::new(rect.x, head.bottom() + 3.0, rect.w, height);
    *rect = rect.below(line + 3.0 + height + 8.0);
    Some(body)
}

/// eDEX-UI's memory grid: 40 × 11 points.
const MEM_COLS: usize = 40;
const MEM_ROWS: usize = 11;

/// Fixed pseudo-random order in which grid points light up (eDEX-UI scatters them).
fn mem_order() -> &'static [usize] {
    static ORDER: std::sync::OnceLock<Vec<usize>> = std::sync::OnceLock::new();
    ORDER.get_or_init(|| {
        let mut v: Vec<usize> = (0..MEM_COLS * MEM_ROWS).collect();
        let mut seed: u64 = 0x5eed_edec;
        for i in (1..v.len()).rev() {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (seed >> 33) as usize % (i + 1);
            v.swap(i, j);
        }
        v
    })
}

fn fmt_gib(kb: u64) -> String {
    format!("{:.1} GiB", kb as f64 / (1024.0 * 1024.0))
}

fn temp_color(t: &crate::theme::Theme, c: f32) -> crate::geometry::Color {
    if c >= 85.0 {
        t.error
    } else if c >= 70.0 {
        t.warning
    } else {
        t.text_primary
    }
}

fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m")
    } else {
        format!("{h:02}h {m:02}m")
    }
}

pub fn draw(ctx: &mut Ctx, rect: Rect, state: &ShellState) {
    let t = ctx.theme;
    let mut area = ctx.frame(rect, Some("SYSTEM"));
    let line = ctx.line();
    let small = ctx.small();
    let si = &state.sysinfo;
    let cw = ctx.metrics.cell_w * 0.9;

    // Identity
    if let Some(body) = section(ctx, &mut area, "HOST", line * 3.0 + 4.0) {
        ctx.label_small(
            Rect::new(body.x, body.y, body.w, line),
            &format!("{}  up {}", state.hostname, fmt_uptime(si.uptime_secs)),
            t.text_primary,
        );
        ctx.label_small(
            Rect::new(body.x, body.y + line, body.w, line),
            &si.kernel,
            t.text_secondary,
        );
        ctx.label_small(
            Rect::new(body.x, body.y + line * 2.0, body.w, line),
            &format!(
                "load {:.2} {:.2} {:.2}",
                si.load_avg[0], si.load_avg[1], si.load_avg[2]
            ),
            t.text_secondary,
        );
    }

    // CPU
    let cores = si.cpu_cores.len().max(1);
    let per_row = if area.w > 260.0 { 4 } else { 2 };
    let core_rows = cores.div_ceil(per_row);
    let bar_h = 8.0;
    let cpu_h = line * 2.0 + core_rows as f32 * (bar_h + 4.0) + 4.0;
    if let Some(body) = section(ctx, &mut area, "CPU", cpu_h) {
        let avg = if si.cpu_cores.is_empty() {
            0.0
        } else {
            si.cpu_cores.iter().sum::<f32>() / cores as f32
        };
        let model: String = si.cpu_model.chars().take((body.w / cw) as usize).collect();
        ctx.label_small(
            Rect::new(body.x, body.y, body.w, line),
            &model,
            t.text_secondary,
        );
        let mut info = format!("{avg:.0}%  {} MHz", si.cpu_freq_mhz);
        if let Some(temp) = si.cpu_temp_c {
            info.push_str(&format!("  {temp:.0}°C"));
        }
        ctx.label_small(
            Rect::new(body.x, body.y + line, body.w, line),
            &info,
            t.text_primary,
        );
        let gap = 6.0;
        let bw = ((body.w - gap * (per_row as f32 - 1.0)) / per_row as f32).floor();
        for (i, pct) in si.cpu_cores.iter().enumerate() {
            let r = i / per_row;
            let c = i % per_row;
            let br = Rect::new(
                body.x + c as f32 * (bw + gap),
                body.y + line * 2.0 + r as f32 * (bar_h + 4.0),
                bw,
                bar_h,
            );
            ctx.meter(br, pct / 100.0, t.border);
        }
    }

    // Memory: the eDEX-UI point grid. Used memory lights points brightly, reclaimable cache
    // dimly; the lit points are scattered, as in eDEX-UI.
    let cols = MEM_COLS;
    let pitch = (area.w / cols as f32).floor().max(3.0);
    let grid_h = pitch * MEM_ROWS as f32;
    if let Some(body) = section(ctx, &mut area, "MEMORY", line * 2.0 + grid_h + bar_h + 12.0) {
        ctx.scene.text_aligned(
            Rect::new(body.x, body.y, body.w, line),
            small,
            t.text_primary,
            Align::Left,
            format!(
                "USING {} OUT OF {}",
                fmt_gib(si.ram_used_kb),
                fmt_gib(si.ram_total_kb)
            ),
        );
        let total = si.ram_total_kb.max(1) as f32;
        let points = MEM_COLS * MEM_ROWS;
        let used = ((si.ram_used_kb as f32 / total) * points as f32).round() as usize;
        let cached = ((si.ram_cached_kb as f32 / total) * points as f32).round() as usize;
        let dot = (pitch * 0.62).round().max(2.0);
        let gx = body.x + (body.w - pitch * cols as f32) / 2.0;
        let gy = body.y + line + 2.0;
        for (rank, &cell) in mem_order().iter().enumerate() {
            let color = if rank < used {
                t.border
            } else if rank < used + cached {
                with_alpha(t.border, 0.38)
            } else {
                with_alpha(t.border, 0.1)
            };
            let (r, c) = (cell / cols, cell % cols);
            ctx.scene.fill(
                Rect::new(
                    gx + c as f32 * pitch + (pitch - dot) / 2.0,
                    gy + r as f32 * pitch + (pitch - dot) / 2.0,
                    dot,
                    dot,
                ),
                color,
            );
        }
        let swap = if si.swap_total_kb > 0 {
            si.swap_used_kb as f32 / si.swap_total_kb as f32
        } else {
            0.0
        };
        let sy = gy + grid_h + 4.0;
        ctx.label_small(Rect::new(body.x, sy, 44.0, line), "SWAP", t.text_secondary);
        ctx.meter(
            Rect::new(
                body.x + 46.0,
                sy + (line - bar_h) / 2.0,
                body.w - 46.0 - 70.0,
                bar_h,
            ),
            swap,
            t.accent,
        );
        ctx.scene.text_aligned(
            Rect::new(body.right() - 68.0, sy, 68.0, line),
            small,
            t.text_secondary,
            Align::Right,
            fmt_gib(si.swap_used_kb),
        );
    }

    // GPUs: one block per card.
    for (i, g) in si.gpus.iter().enumerate() {
        let no_data = g.busy_pct.is_none()
            && g.vram_total_bytes.is_none()
            && g.clock_mhz.is_none()
            && g.power_w.is_none()
            && g.temp_c.is_none();
        let rows = 1
            + usize::from(no_data)
            + usize::from(g.busy_pct.is_some())
            + usize::from(g.vram_total_bytes.is_some())
            + usize::from(g.clock_mhz.is_some() || g.power_w.is_some());
        let title = if si.gpus.len() > 1 {
            format!("GPU {i}")
        } else {
            "GPU".to_string()
        };
        let Some(body) = section(ctx, &mut area, &title, rows as f32 * line) else {
            break;
        };
        let mut y = body.y;
        let temp_w = 52.0;
        let name: String = g
            .name
            .chars()
            .take(((body.w - temp_w) / cw).max(6.0) as usize)
            .collect();
        ctx.label_small(
            Rect::new(body.x, y, body.w - temp_w, line),
            &name,
            t.text_primary,
        );
        if let Some(temp) = g.temp_c {
            ctx.scene.text_aligned(
                Rect::new(body.right() - temp_w, y, temp_w, line),
                small,
                temp_color(t, temp),
                Align::Right,
                format!("{temp:.0}°C"),
            );
        }
        y += line;
        let meter_row = |ctx: &mut Ctx, y: f32, label: &str, frac: f32, value: String| {
            ctx.label_small(Rect::new(body.x, y, 44.0, line), label, t.text_secondary);
            ctx.meter(
                Rect::new(
                    body.x + 46.0,
                    y + (line - bar_h) / 2.0,
                    body.w - 46.0 - 90.0,
                    bar_h,
                ),
                frac,
                t.border,
            );
            ctx.scene.text_aligned(
                Rect::new(body.right() - 88.0, y, 88.0, line),
                small,
                t.text_secondary,
                Align::Right,
                value,
            );
        };
        if let Some(busy) = g.busy_pct {
            meter_row(ctx, y, "LOAD", busy / 100.0, format!("{busy:.0}%"));
            y += line;
        }
        if let (Some(used), Some(total)) = (g.vram_used_bytes, g.vram_total_bytes) {
            let frac = used as f32 / total.max(1) as f32;
            meter_row(
                ctx,
                y,
                "VRAM",
                frac,
                format!("{}/{}", fmt_gib(used / 1024), fmt_gib(total / 1024)),
            );
            y += line;
        }
        if no_data {
            ctx.label_small(
                Rect::new(body.x, y, body.w, line),
                "no sensors exposed by the driver",
                t.text_dim,
            );
        }
        if g.clock_mhz.is_some() || g.power_w.is_some() {
            let mut info = String::new();
            if let Some(c) = g.clock_mhz {
                info.push_str(&format!("{c} MHz"));
            }
            if let Some(p) = g.power_w {
                if !info.is_empty() {
                    info.push_str("  ");
                }
                info.push_str(&format!("{p:.0} W"));
            }
            ctx.label_small(Rect::new(body.x, y, body.w, line), &info, t.text_secondary);
        }
    }

    // Temperatures: two columns, hottest first.
    let shown = si.temps.len().min(6);
    if shown > 0 {
        let rows = shown.div_ceil(2);
        if let Some(body) = section(ctx, &mut area, "TEMPERATURES", rows as f32 * line) {
            let col_w = (body.w - 12.0) / 2.0;
            for (i, (label, temp)) in si.temps.iter().take(shown).enumerate() {
                let x = body.x + (i % 2) as f32 * (col_w + 12.0);
                let y = body.y + (i / 2) as f32 * line;
                let name: String = label
                    .chars()
                    .take(((col_w - 44.0) / cw).max(3.0) as usize)
                    .collect();
                ctx.label_small(Rect::new(x, y, col_w - 44.0, line), &name, t.text_secondary);
                ctx.scene.text_aligned(
                    Rect::new(x + col_w - 44.0, y, 44.0, line),
                    small,
                    temp_color(t, *temp),
                    Align::Right,
                    format!("{temp:.0}°"),
                );
            }
        }
    }

    // Network
    let graph_h = 24.0;
    if let Some(body) = section(ctx, &mut area, "NETWORK", line * 2.0 + graph_h * 2.0 + 8.0) {
        let iface = if si.net_iface.is_empty() {
            "no link".to_string()
        } else {
            format!("{}  {}", si.net_iface, si.net_ip)
        };
        ctx.label_small(
            Rect::new(body.x, body.y, body.w, line),
            &iface,
            t.text_secondary,
        );
        let peak = si
            .net_tx_history
            .iter()
            .chain(si.net_rx_history.iter())
            .cloned()
            .fold(1.0f32, f32::max);
        const SLOTS: usize = 60;
        let norm = |v: &[f32]| {
            let mut out = vec![0.0f32; SLOTS.saturating_sub(v.len())];
            out.extend(v.iter().rev().take(SLOTS).rev().map(|x| x / peak));
            out
        };
        let tx = Rect::new(body.x, body.y + line, body.w, graph_h);
        ctx.scene.sparkline(
            tx,
            &norm(&si.net_tx_history),
            with_alpha(t.accent, 0.8),
            with_alpha(t.border, 0.08),
        );
        ctx.scene.text_aligned(
            Rect::new(tx.x + 4.0, tx.y, tx.w - 8.0, line),
            small,
            t.text_dim,
            Align::Right,
            format!("▲ {:.0} kb/s", si.net_tx_kbps),
        );
        let rx = Rect::new(body.x, tx.bottom() + 4.0, body.w, graph_h);
        ctx.scene.sparkline(
            rx,
            &norm(&si.net_rx_history),
            with_alpha(t.border, 0.8),
            with_alpha(t.border, 0.08),
        );
        ctx.scene.text_aligned(
            Rect::new(rx.x + 4.0, rx.y, rx.w - 8.0, line),
            small,
            t.text_dim,
            Align::Right,
            format!("▼ {:.0} kb/s", si.net_rx_kbps),
        );
    }

    // Disks
    let disks = si.disks.len().min(4);
    if disks > 0 {
        if let Some(body) = section(
            ctx,
            &mut area,
            "STORAGE",
            disks as f32 * (line + bar_h + 2.0),
        ) {
            for (i, d) in si.disks.iter().take(disks).enumerate() {
                let y = body.y + i as f32 * (line + bar_h + 2.0);
                let mount: String = d.mount.chars().take(18).collect();
                ctx.label_small(
                    Rect::new(body.x, y, body.w * 0.5, line),
                    &mount,
                    t.text_primary,
                );
                ctx.scene.text_aligned(
                    Rect::new(body.x + body.w * 0.5, y, body.w * 0.5, line),
                    small,
                    t.text_secondary,
                    Align::Right,
                    format!("{} / {}", d.used_str, d.total_str),
                );
                ctx.meter(
                    Rect::new(body.x, y + line, body.w, bar_h),
                    d.used_pct,
                    t.border,
                );
            }
        }
    }

    // Processes: fill remaining space
    let remaining = area.h - line - 12.0;
    let proc_rows = ((remaining / line).floor().max(0.0) as usize).min(si.processes.len());
    if proc_rows > 0 {
        if let Some(body) = section(ctx, &mut area, "PROCESSES", proc_rows as f32 * line) {
            for (i, p) in si.processes.iter().take(proc_rows).enumerate() {
                let y = body.y + i as f32 * line;
                let name: String = p
                    .name
                    .chars()
                    .take(((body.w - 110.0) / cw).max(4.0) as usize)
                    .collect();
                ctx.label_small(
                    Rect::new(body.x, y, 56.0, line),
                    &p.pid.to_string(),
                    t.text_dim,
                );
                ctx.label_small(
                    Rect::new(body.x + 56.0, y, body.w - 166.0, line),
                    &name,
                    t.text_primary,
                );
                ctx.scene.text_aligned(
                    Rect::new(body.right() - 110.0, y, 50.0, line),
                    small,
                    t.text_secondary,
                    Align::Right,
                    format!("{:.0}%", p.cpu_pct),
                );
                ctx.scene.text_aligned(
                    Rect::new(body.right() - 56.0, y, 56.0, line),
                    small,
                    t.text_secondary,
                    Align::Right,
                    p.mem_str.clone(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_grid_order_is_a_permutation() {
        let mut o = mem_order().to_vec();
        assert_ne!(o[..10], (0..10).collect::<Vec<_>>()[..]);
        o.sort();
        assert_eq!(o, (0..MEM_COLS * MEM_ROWS).collect::<Vec<_>>());
    }
}
