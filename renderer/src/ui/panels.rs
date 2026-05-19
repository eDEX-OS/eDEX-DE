use super::resize::ResizeState;

#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PanelLayout {
    pub filesystem: Rectangle,
    pub terminal: Rectangle,
    pub sysinfo: Rectangle,
    pub keyboard: Rectangle,
    pub top_bar: Rectangle,
    pub status_bar: Rectangle,
    pub fs_terminal_handle: Rectangle,
    pub terminal_sysinfo_handle: Rectangle,
}

impl PanelLayout {
    pub fn from_size(width: u32, height: u32) -> Self {
        Self::from_state(width, height, ResizeState::default())
    }

    pub fn from_state(width: u32, height: u32, resize: ResizeState) -> Self {
        let status_bar_h = 30;
        let top_bar_h = 40;
        let keyboard_h = (height / 5).max(120);
        let panel_y = status_bar_h + top_bar_h;
        let panel_h = height.saturating_sub(panel_y + keyboard_h);

        let min_fs = 180;
        let min_terminal = 320;
        let min_sysinfo = 220;
        let max_fs_boundary = width.saturating_sub(min_terminal + min_sysinfo).max(min_fs);
        let fs_boundary = ((width as f32) * resize.fs_terminal_split)
            .round()
            .clamp(min_fs as f32, max_fs_boundary as f32) as u32;
        let min_terminal_boundary = fs_boundary.saturating_add(min_terminal);
        let max_terminal_boundary = width.saturating_sub(min_sysinfo).max(min_terminal_boundary);
        let terminal_boundary = ((width as f32) * resize.terminal_sysinfo_split)
            .round()
            .clamp(min_terminal_boundary as f32, max_terminal_boundary as f32)
            as u32;

        let handle_w = 4;
        let filesystem = Rectangle::new(0, panel_y, fs_boundary, panel_h);
        let terminal = Rectangle::new(
            fs_boundary,
            panel_y,
            terminal_boundary.saturating_sub(fs_boundary),
            panel_h,
        );
        let sysinfo = Rectangle::new(
            terminal_boundary,
            panel_y,
            width.saturating_sub(terminal_boundary),
            panel_h,
        );

        Self {
            status_bar: Rectangle::new(0, 0, width, status_bar_h),
            top_bar: Rectangle::new(0, status_bar_h, width, top_bar_h),
            filesystem,
            terminal,
            sysinfo,
            keyboard: Rectangle::new(0, height.saturating_sub(keyboard_h), width, keyboard_h),
            fs_terminal_handle: Rectangle::new(
                fs_boundary.saturating_sub(handle_w / 2),
                panel_y,
                handle_w,
                panel_h,
            ),
            terminal_sysinfo_handle: Rectangle::new(
                terminal_boundary.saturating_sub(handle_w / 2),
                panel_y,
                handle_w,
                panel_h,
            ),
        }
    }
}
