//! Filesystem panel drawing.

use crate::{
    filesystem::{format_size, icon_for_entry},
    geometry::{with_alpha, Rect},
    hit::HitTarget,
    state::{PanelFocus, ShellState},
    widgets::Ctx,
};

pub fn draw(ctx: &mut Ctx, rect: Rect, state: &ShellState) {
    let t = ctx.theme;
    let focused = state.focus == PanelFocus::Filesystem && state.shell_focused;
    let inner = ctx.frame(
        rect,
        Some(if focused {
            "FILESYSTEM ●"
        } else {
            "FILESYSTEM"
        }),
    );
    ctx.hits.push(rect, HitTarget::FilesystemArea);
    let line = ctx.line();
    // "RANGER" in the frame header opens ranger in a terminal tab here.
    let head_h = (inner.y - rect.y - 2.0).max(line * 0.8);
    let ranger = Rect::new(rect.right() - 74.0, rect.y + 1.0, 70.0, head_h);
    ctx.scene.fill(ranger, with_alpha(t.border, 0.14));
    ctx.scene.text_aligned(
        Rect::new(ranger.x, ranger.y + (ranger.h - line) / 2.0, ranger.w, line),
        ctx.small(),
        t.border,
        crate::scene::Align::Center,
        "RANGER ▸",
    );
    ctx.hits.push(ranger, HitTarget::FsRanger);
    let fs = &state.filesystem;

    // Breadcrumbs
    let crumbs = fs.breadcrumbs();
    let mut x = inner.x;
    let crumb_y = inner.y;
    let cw = ctx.metrics.cell_w * 0.9;
    let max_x = inner.right();
    // Show only the trailing crumbs that fit.
    let mut widths: Vec<f32> = crumbs
        .iter()
        .map(|c| (c.chars().count() as f32 * cw).round() + 10.0)
        .collect();
    let mut start = 0;
    while start < crumbs.len() && widths[start..].iter().sum::<f32>() + 20.0 > inner.w {
        start += 1;
    }
    if start > 0 {
        ctx.label_small(Rect::new(x, crumb_y, 16.0, line), "…", t.text_dim);
        x += 16.0;
    }
    for (i, crumb) in crumbs.iter().enumerate().skip(start) {
        let w = widths[i].min(max_x - x);
        if w <= 0.0 {
            break;
        }
        let r = Rect::new(x, crumb_y, w, line);
        let last = i + 1 == crumbs.len();
        ctx.scene
            .fill_rounded(r, with_alpha(t.border, if last { 0.25 } else { 0.1 }), 2.0);
        ctx.label_small(
            Rect::new(r.x + 5.0, r.y, r.w - 5.0, line),
            crumb,
            if last {
                t.text_primary
            } else {
                t.text_secondary
            },
        );
        ctx.hits.push(r, HitTarget::FsBreadcrumb(i));
        x += w + 4.0;
    }
    widths.clear();

    let list = Rect::new(inner.x, crumb_y + line + 6.0, inner.w, inner.h - line - 6.0);
    ctx.scene
        .hline(list.x, list.y - 3.0, list.w, with_alpha(t.border, 0.3));
    let row_h = line;
    let rows = (list.h / row_h).floor().max(1.0) as usize;
    let _ = rows;

    if let Some(err) = &fs.error {
        ctx.scene.paragraph(
            list,
            ctx.metrics.ui_font,
            t.error,
            format!("cannot read directory: {err}"),
        );
        return;
    }

    let selected_visible = fs.selected_visible_index();
    let mut y = list.y;
    for (i, entry) in fs.visible_entries().iter().enumerate() {
        if y + row_h > list.bottom() + 0.5 {
            break;
        }
        let r = Rect::new(list.x, y, list.w, row_h);
        let selected = selected_visible == Some(i);
        if selected {
            ctx.scene
                .fill(r, with_alpha(t.border, if focused { 0.28 } else { 0.14 }));
            ctx.scene.fill(Rect::new(r.x, r.y, 2.0, r.h), t.border);
        }
        ctx.hits.push(r, HitTarget::FsEntry(i));
        let icon = icon_for_entry(entry);
        let color = if entry.is_dir {
            t.border
        } else if entry.is_hidden {
            t.text_dim
        } else {
            t.text_primary
        };
        let size_text = entry.size.map(format_size).unwrap_or_default();
        let size_w = (size_text.chars().count() as f32 * cw).round() + 4.0;
        ctx.label_small(
            Rect::new(r.x + 6.0, r.y, 18.0, row_h),
            icon,
            if entry.is_dir {
                t.border
            } else {
                t.text_secondary
            },
        );
        ctx.label_small(
            Rect::new(r.x + 24.0, r.y, (r.w - 28.0 - size_w).max(10.0), row_h),
            &entry.name,
            color,
        );
        if !size_text.is_empty() {
            let s = ctx.small();
            ctx.scene.text_aligned(
                Rect::new(r.right() - size_w - 4.0, r.y, size_w, row_h),
                s,
                t.text_dim,
                crate::scene::Align::Right,
                size_text,
            );
        }
        y += row_h;
    }
    if fs.entries.is_empty() {
        ctx.label_small(
            Rect::new(list.x + 6.0, list.y, list.w, row_h),
            "(empty)",
            t.text_dim,
        );
    }
    // Scroll indicator
    if fs.entries.len() > fs.max_visible {
        let track = Rect::new(list.right() - 3.0, list.y, 3.0, list.h);
        ctx.scene.fill(track, with_alpha(t.border, 0.15));
        let frac_h = (fs.max_visible as f32 / fs.entries.len() as f32).clamp(0.05, 1.0);
        let frac_y = fs.scroll_offset as f32 / fs.entries.len() as f32;
        ctx.scene.fill(
            Rect::new(track.x, track.y + track.h * frac_y, 3.0, track.h * frac_h),
            with_alpha(t.border, 0.7),
        );
    }
}
