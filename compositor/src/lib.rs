//! eDEX-DE Wayland compositor.
//!
//! Built on smithay — provides DRM/KMS backend, libinput, Wayland protocol handling,
//! and the tiling window manager logic.

pub mod backend;
pub mod shell;
pub mod state;

pub use state::{
    run_compositor, run_compositor_with_socket_notifier,
    run_compositor_with_socket_notifier_and_launcher_flag,
    run_compositor_with_socket_notifier_and_shared_flags, CalloopData, CompositorState, EdexState,
};
