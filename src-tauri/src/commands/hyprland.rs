use crate::hyprland::ipc;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HyprlandStatus {
    pub running: bool,
    pub instance: Option<String>,
}

#[tauri::command]
pub fn get_hyprland_status() -> HyprlandStatus {
    let instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    HyprlandStatus {
        running: instance.is_some(),
        instance,
    }
}

#[tauri::command]
pub async fn get_workspaces() -> Result<Vec<ipc::WorkspaceInfo>, String> {
    ipc::get_workspaces().await
}

#[tauri::command]
pub async fn get_active_window() -> Result<Option<ipc::ActiveWindowInfo>, String> {
    ipc::get_active_window().await
}

#[tauri::command]
pub async fn get_monitors() -> Result<Vec<ipc::MonitorInfo>, String> {
    ipc::get_monitors().await
}

#[tauri::command]
pub async fn switch_workspace(id: i32) -> Result<(), String> {
    ipc::switch_workspace(id).await
}

#[tauri::command]
pub async fn hypr_dispatch(action: String) -> Result<String, String> {
    ipc::dispatch(&action).await
}

#[tauri::command]
pub fn generate_hyprland_config() -> String {
    r#"# eDEX-DE — Hyprland Configuration
# Add these lines to your ~/.config/hypr/hyprland.conf

# Launch eDEX-DE on startup
exec-once = edex-de

# App launcher (Meta/Super key)
bind = SUPER, SUPER, exec, edex-de --launcher
bind = ALT, Space, exec, edex-de --launcher

# eDEX-DE window rules (keeps it as an overlay)
windowrulev2 = float, class:^(edex-de)$
windowrulev2 = nofocus, class:^(edex-de)$, title:^(eDEX-DE Launcher)$
windowrulev2 = pin, class:^(edex-de)$

# Workspace switching (if not already set)
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, 3, workspace, 3
bind = SUPER, 4, workspace, 4
bind = SUPER, 5, workspace, 5
bind = SUPER, 6, workspace, 6
bind = SUPER, 7, workspace, 7
bind = SUPER, 8, workspace, 8
bind = SUPER, 9, workspace, 9
bind = SUPER, 0, workspace, 10
"#
    .to_string()
}
