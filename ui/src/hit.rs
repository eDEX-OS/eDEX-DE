//! Hit regions registered while building a scene, resolved on pointer events.

use crate::geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    FsTerminal,
    TerminalSysinfo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusItem {
    Tor,
    Tailscale,
    Vpn,
    WireGuard,
    Fingerprint,
    Microphone,
    Camera,
    Volume,
    Battery,
    Network,
    Notifications,
}

/// What a pointer position resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    None,
    TerminalArea,
    TerminalTab(usize),
    TerminalTabClose(usize),
    TerminalNewTab,
    /// Application window tab in the centre strip (index into `ShellState::windows`).
    AppTab(usize),
    AppTabClose(usize),
    /// Controls for the focused application window.
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    FilesystemArea,
    FsEntry(usize),
    FsBreadcrumb(usize),
    FsParent,
    /// Open ranger in the file panel's directory.
    FsRanger,
    KeyboardKey(usize, usize),
    ResizeHandle(ResizeHandle),
    Workspace(u32),
    Install,
    Launcher,
    Status(StatusItem),
    Clock,
    /// Overlay widgets (launcher items, settings controls, ...).
    OverlayItem(u32),
    OverlayClose,
    /// Absorbs clicks so they do not close the overlay.
    OverlayPanel,
    Toast(u32),
    ToastAction(u32),
}

#[derive(Clone, Debug, Default)]
pub struct HitMap {
    entries: Vec<(Rect, HitTarget)>,
}

impl HitMap {
    pub fn push(&mut self, rect: Rect, target: HitTarget) {
        self.entries.push((rect, target));
    }

    /// Later entries win (drawn on top).
    pub fn resolve(&self, x: f32, y: f32) -> HitTarget {
        self.entries
            .iter()
            .rev()
            .find(|(r, _)| r.contains(x, y))
            .map(|(_, t)| *t)
            .unwrap_or(HitTarget::None)
    }

    pub fn rect_of(&self, target: HitTarget) -> Option<Rect> {
        self.entries
            .iter()
            .find(|(_, t)| *t == target)
            .map(|(r, _)| *r)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topmost_wins() {
        let mut map = HitMap::default();
        map.push(Rect::new(0.0, 0.0, 100.0, 100.0), HitTarget::TerminalArea);
        map.push(Rect::new(10.0, 10.0, 20.0, 20.0), HitTarget::TerminalTab(1));
        assert_eq!(map.resolve(15.0, 15.0), HitTarget::TerminalTab(1));
        assert_eq!(map.resolve(50.0, 50.0), HitTarget::TerminalArea);
        assert_eq!(map.resolve(500.0, 50.0), HitTarget::None);
    }
}
