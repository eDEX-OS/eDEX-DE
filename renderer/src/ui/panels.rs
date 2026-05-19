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
}

impl PanelLayout {
    pub fn from_size(width: u32, height: u32) -> Self {
        let status_bar_h = 30;
        let top_bar_h = 40;
        let keyboard_h = (height / 5).max(120);
        let panel_y = status_bar_h + top_bar_h;
        let panel_h = height.saturating_sub(panel_y + keyboard_h);

        let filesystem_w = width / 5;
        let terminal_w = width / 2;
        let sysinfo_w = width.saturating_sub(filesystem_w + terminal_w);

        Self {
            status_bar: Rectangle::new(0, 0, width, status_bar_h),
            top_bar: Rectangle::new(0, status_bar_h, width, top_bar_h),
            filesystem: Rectangle::new(0, panel_y, filesystem_w, panel_h),
            terminal: Rectangle::new(filesystem_w, panel_y, terminal_w, panel_h),
            sysinfo: Rectangle::new(filesystem_w + terminal_w, panel_y, sysinfo_w, panel_h),
            keyboard: Rectangle::new(0, height.saturating_sub(keyboard_h), width, keyboard_h),
        }
    }
}
