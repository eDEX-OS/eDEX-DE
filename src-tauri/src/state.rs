use crate::pty::server::PtyHandle;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub pty_sessions: Mutex<HashMap<u16, PtyHandle>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            pty_sessions: Mutex::new(HashMap::new()),
        })
    }
}
