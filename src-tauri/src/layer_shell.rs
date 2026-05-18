//! Attempts to initialize gtk-layer-shell for the main window.
//! Gracefully no-ops if the library is not installed.

/// Try to apply layer-shell-related integration to the main window.
///
/// This currently performs an availability probe for gtk-layer-shell and logs the
/// outcome. Direct GTK window integration is deferred until a deeper Tauri/GTK
/// bridge is added, so Hyprland window rules remain the active integration path.
pub fn try_apply_layer_shell(window: &tauri::WebviewWindow) {
    let _ = window;

    #[cfg(target_os = "linux")]
    {
        let candidate_paths = [
            "/usr/lib/libgtk-layer-shell.so",
            "/usr/lib/libgtk-layer-shell.so.0",
            "/usr/lib/x86_64-linux-gnu/libgtk-layer-shell.so",
            "/usr/lib/x86_64-linux-gnu/libgtk-layer-shell.so.0",
            "/usr/lib/aarch64-linux-gnu/libgtk-layer-shell.so",
            "/usr/lib/aarch64-linux-gnu/libgtk-layer-shell.so.0",
            "/usr/lib64/libgtk-layer-shell.so",
            "/usr/lib64/libgtk-layer-shell.so.0",
        ];

        let available = candidate_paths
            .iter()
            .any(|path| std::path::Path::new(path).exists());

        if available {
            tracing::info!("[eDEX-DE] gtk-layer-shell detected; using Hyprland rules for DE positioning");
        } else {
            tracing::warn!("[eDEX-DE] gtk-layer-shell not found; using Hyprland rules for DE positioning");
        }
    }
}
