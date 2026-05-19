//! eDEX-DE terminal emulator.
//!
//! PTY management via portable-pty, VT100/ANSI parsing via vte,
//! and a shared cell grid for the renderer.

pub mod clipboard;
pub mod grid;
pub mod input;
pub mod instance;
pub mod tab;

pub use clipboard::Clipboard;
pub use grid::{Cell, CellAttributes, TermColor, TerminalGrid};
pub use input::{key_event_to_bytes, keysym_to_bytes, Modifiers};
pub use instance::TerminalInstance;
pub use tab::TerminalTabs;
