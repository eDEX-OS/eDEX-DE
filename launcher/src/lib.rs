//! eDEX-DE app launcher.
//!
//! Scans XDG .desktop files, fuzzy-searches apps, launches via exec.
//! Triggered by Alt+Space overlay rendered by the wgpu renderer.

pub mod desktop;
pub mod runner;
pub mod search;

pub use desktop::{scan_applications, AppEntry};
pub use runner::launch_app;
pub use search::AppSearch;
