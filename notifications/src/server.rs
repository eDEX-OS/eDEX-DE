//! D-Bus server implementing the Desktop Notifications Specification 1.2.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use anyhow::{anyhow, Context, Result};
use tracing::{info, warn};
use zbus::{
    blocking::Connection,
    fdo::RequestNameFlags,
    interface,
    object_server::SignalEmitter,
    zvariant::{OwnedValue, Value},
};

use crate::store::{Notification, Urgency};

pub const CLOSE_EXPIRED: u32 = 1;
pub const CLOSE_DISMISSED: u32 = 2;
pub const CLOSE_REQUESTED: u32 = 3;

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

#[derive(Clone, Debug)]
pub enum ServerEvent {
    New(Notification),
    CloseRequested(u32),
}

pub type EventSink = Arc<dyn Fn(ServerEvent) + Send + Sync>;

struct Interface {
    sink: EventSink,
    next_id: Arc<AtomicU32>,
}

fn hint_u8(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<u8> {
    let v = hints.get(key)?;
    match &**v {
        Value::U8(b) => Some(*b),
        Value::I32(i) => Some((*i).clamp(0, 2) as u8),
        Value::U32(i) => Some((*i).min(2) as u8),
        _ => None,
    }
}

fn hint_i32(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<i32> {
    let v = hints.get(key)?;
    match &**v {
        Value::I32(i) => Some(*i),
        Value::U32(i) => Some(*i as i32),
        Value::I64(i) => Some(*i as i32),
        Value::U8(i) => Some(*i as i32),
        _ => None,
    }
}

fn hint_bool(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    let v = hints.get(key)?;
    match &**v {
        Value::Bool(b) => Some(*b),
        Value::I32(i) => Some(*i != 0),
        Value::U8(i) => Some(*i != 0),
        _ => None,
    }
}

fn hint_str(hints: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let v = hints.get(key)?;
    match &**v {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

/// Strip the limited markup the spec allows (`<b>`, `<i>`, `<u>`, `<a>`, `<img>`) to plain text.
pub fn strip_markup(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_tag = false;
    for c in body.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[interface(name = "org.freedesktop.Notifications")]
impl Interface {
    fn get_capabilities(&self) -> Vec<&'static str> {
        vec![
            "body",
            "body-markup",
            "actions",
            "persistence",
            "action-icons",
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::SeqCst).max(1)
        };
        let actions: Vec<(String, String)> = actions
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| (c[0].to_string(), c[1].to_string()))
            .collect();
        let notification = Notification {
            id,
            app: if app_name.is_empty() {
                "system".into()
            } else {
                app_name.to_string()
            },
            icon: app_icon.to_string(),
            summary: summary.to_string(),
            body: strip_markup(body),
            actions,
            urgency: Urgency::from(hint_u8(&hints, "urgency").unwrap_or(1)),
            timeout_ms: if expire_timeout < 0 {
                None
            } else {
                Some(expire_timeout as u32)
            },
            progress: hint_i32(&hints, "value").map(|v| (v as f32 / 100.0).clamp(0.0, 1.0)),
            transient: hint_bool(&hints, "transient").unwrap_or(false),
            desktop_entry: hint_str(&hints, "desktop-entry"),
            created: Instant::now(),
            wall_time: 0,
        };
        (self.sink)(ServerEvent::New(notification));
        id
    }

    fn close_notification(&self, id: u32) {
        (self.sink)(ServerEvent::CloseRequested(id));
    }

    fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("edex-de", "eDEX-OS", env!("CARGO_PKG_VERSION"), "1.2")
    }

    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

/// Running server; drop to release the bus name.
pub struct NotificationServer {
    connection: Connection,
    _next_id: Arc<AtomicU32>,
    owned: Arc<Mutex<bool>>,
}

impl NotificationServer {
    /// Own `org.freedesktop.Notifications` on the session bus. Fails when another daemon
    /// already owns the name (we never queue behind it).
    pub fn start(sink: EventSink) -> Result<Self> {
        let next_id = Arc::new(AtomicU32::new(1));
        let connection = Connection::session().context("connecting to the session bus")?;
        connection
            .object_server()
            .at(
                OBJECT_PATH,
                Interface {
                    sink,
                    next_id: next_id.clone(),
                },
            )
            .context("registering the notifications object")?;
        match connection.request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into()) {
            Ok(_) => {
                info!("owning {BUS_NAME}");
                Ok(Self {
                    connection,
                    _next_id: next_id,
                    owned: Arc::new(Mutex::new(true)),
                })
            }
            Err(e) => {
                warn!("could not own {BUS_NAME}: {e}; another notification daemon is running");
                Err(anyhow!("{BUS_NAME} is owned by another daemon"))
            }
        }
    }

    pub fn is_owner(&self) -> bool {
        *self.owned.lock().unwrap()
    }

    fn emitter(&self) -> Result<SignalEmitter<'static>> {
        Ok(SignalEmitter::new(self.connection.inner(), OBJECT_PATH)?.to_owned())
    }

    pub fn emit_closed(&self, id: u32, reason: u32) {
        if let Ok(emitter) = self.emitter() {
            let _ = zbus::block_on(Interface::notification_closed(&emitter, id, reason));
        }
    }

    pub fn emit_action(&self, id: u32, key: &str) {
        if let Ok(emitter) = self.emitter() {
            let _ = zbus::block_on(Interface::action_invoked(&emitter, id, key));
        }
    }
}

impl Drop for NotificationServer {
    fn drop(&mut self) {
        let _ = self.connection.release_name(BUS_NAME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };

    #[test]
    fn strips_markup() {
        assert_eq!(
            strip_markup("<b>bold</b> &amp; <a href='x'>link</a>"),
            "bold & link"
        );
    }

    /// Spawns a private session bus, starts the server and sends a notification with
    /// `notify-send`, checking the event arrives and the name is owned.
    #[test]
    fn serves_notifications_on_a_private_bus() {
        if Command::new("dbus-daemon")
            .arg("--version")
            .output()
            .is_err()
            || Command::new("notify-send")
                .arg("--version")
                .output()
                .is_err()
        {
            eprintln!("skipping: dbus-daemon or notify-send not installed");
            return;
        }
        let dir = std::env::temp_dir().join(format!("edex-notif-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let addr = format!("unix:path={}/bus", dir.display());
        let mut daemon = Command::new("dbus-daemon")
            .args([
                "--session",
                "--nofork",
                "--nopidfile",
                &format!("--address={addr}"),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        // Wait for the bus socket rather than a fixed delay: package builds run tests on busy
        // machines, where the daemon can take seconds to come up.
        let socket = dir.join("bus");
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while !socket.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(socket.exists(), "dbus-daemon did not create its socket");
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &addr);

        let (tx, rx) = mpsc::channel();
        let sink: EventSink = Arc::new(move |e| {
            let _ = tx.send(e);
        });
        let server = NotificationServer::start(sink).unwrap();
        assert!(server.is_owner());
        let status = Command::new("notify-send")
            .args([
                "-a",
                "test-app",
                "-u",
                "critical",
                "-t",
                "0",
                "Hello",
                "<b>World</b>",
            ])
            .env("DBUS_SESSION_BUS_ADDRESS", &addr)
            .status()
            .unwrap();
        assert!(status.success());
        let ev = rx.recv_timeout(Duration::from_secs(20)).unwrap();
        match ev {
            ServerEvent::New(n) => {
                assert_eq!(n.app, "test-app");
                assert_eq!(n.summary, "Hello");
                assert_eq!(n.body, "World");
                assert_eq!(n.urgency, Urgency::Critical);
                assert_eq!(n.timeout_ms, Some(0));
                server.emit_closed(n.id, CLOSE_DISMISSED);
            }
            other => panic!("unexpected {other:?}"),
        }
        // A second server must fail to take the name.
        let sink2: EventSink = Arc::new(|_| {});
        assert!(NotificationServer::start(sink2).is_err());
        drop(server);
        let _ = daemon.kill();
        let _ = daemon.wait();
        let _ = std::fs::remove_dir_all(dir);
    }
}
