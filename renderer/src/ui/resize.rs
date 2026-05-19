use super::panels::Rectangle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragTarget {
    FsTerminal,
    TerminalSysinfo,
}

#[derive(Clone, Copy, Debug)]
pub struct ResizeState {
    pub fs_terminal_split: f32,
    pub terminal_sysinfo_split: f32,
    pub dragging: Option<DragTarget>,
}

impl Default for ResizeState {
    fn default() -> Self {
        Self {
            fs_terminal_split: 0.2,
            terminal_sysinfo_split: 0.7,
            dragging: None,
        }
    }
}

impl ResizeState {
    pub fn handle_color_intensity(&self, target: DragTarget) -> f32 {
        match self.dragging {
            Some(active) if active == target => 1.0,
            _ => 0.6,
        }
    }

    pub fn grab_indicator(handle: Rectangle) -> Rectangle {
        let height = handle.height.min(56);
        Rectangle::new(
            handle.x.saturating_sub(1),
            handle.y + handle.height.saturating_sub(height) / 2,
            handle.width + 2,
            height,
        )
    }
}
