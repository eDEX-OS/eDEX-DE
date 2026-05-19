//! eDEX-DE app launcher.
//!
//! Scans XDG .desktop files, fuzzy-searches apps, launches via exec.
//! Triggered by Alt+Space overlay rendered by the wgpu renderer.

pub mod desktop;
pub mod search;
