use crate::pty::server::spawn_pty_server;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn spawn_terminal(
    state: State<'_, Arc<AppState>>,
    port: u16,
    shell: String,
    cwd: String,
    env: Vec<(String, String)>,
) -> Result<u16, String> {
    let handle = spawn_pty_server(port, shell, cwd, env).await?;
    let actual_port = handle.port;
    state.pty_sessions.lock().await.insert(actual_port, handle);
    Ok(actual_port)
}

#[tauri::command]
pub async fn close_terminal(state: State<'_, Arc<AppState>>, port: u16) -> Result<(), String> {
    let mut sessions = state.pty_sessions.lock().await;
    if let Some(handle) = sessions.remove(&port) {
        let _ = handle.shutdown_tx.send(());
        let mut child = handle.child.lock().await;
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

#[tauri::command]
pub async fn list_terminals(state: State<'_, Arc<AppState>>) -> Result<Vec<u16>, String> {
    Ok(state.pty_sessions.lock().await.keys().copied().collect())
}
