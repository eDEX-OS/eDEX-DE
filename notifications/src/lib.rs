//! eDEX-DE in-process notification store.
//!
//! Provides a thread-safe notification queue used by the renderer to display
//! toast overlays. No D-Bus dependency — notifications are pushed in-process.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use tracing::debug;

/// A single notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub created_at: SystemTime,
    pub timeout: Duration,
}

impl Notification {
    pub fn is_expired(&self) -> bool {
        self.created_at
            .elapsed()
            .map(|e| e > self.timeout)
            .unwrap_or(false)
    }
}

/// Thread-safe notification store (max 10 active notifications).
#[derive(Debug, Clone)]
pub struct NotificationStore {
    inner: Arc<Mutex<VecDeque<Notification>>>,
    next_id: Arc<Mutex<u32>>,
}

impl Default for NotificationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(10))),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Push a new notification and return its ID.
    pub fn push(&self, app: &str, summary: &str, body: &str, timeout_ms: u32) -> u32 {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            let id = *n;
            *n = n.wrapping_add(1).max(1);
            id
        };
        let notif = Notification {
            id,
            app_name: app.to_string(),
            summary: summary.to_string(),
            body: body.to_string(),
            created_at: SystemTime::now(),
            timeout: Duration::from_millis(u64::from(timeout_ms)),
        };
        debug!(id, summary, "notification pushed");
        let mut q = self.inner.lock().unwrap();
        if q.len() >= 10 {
            q.pop_front();
        }
        q.push_back(notif);
        id
    }

    /// Dismiss a notification by ID.
    pub fn dismiss(&self, id: u32) {
        let mut q = self.inner.lock().unwrap();
        q.retain(|n| n.id != id);
    }

    /// Return all non-expired notifications (and prune expired ones).
    pub fn active(&self) -> Vec<Notification> {
        let mut q = self.inner.lock().unwrap();
        q.retain(|n| !n.is_expired());
        q.iter().cloned().collect()
    }

    /// Clear all notifications.
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}
