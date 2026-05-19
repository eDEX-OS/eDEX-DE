use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct Clipboard {
    fallback: Arc<Mutex<String>>,
}

impl Clipboard {
    pub fn copy(&self, text: &str) {
        if let Ok(mut fallback) = self.fallback.lock() {
            fallback.clear();
            fallback.push_str(text);
        }

        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    }

    pub fn paste(&self) -> Option<String> {
        if let Ok(output) = Command::new("wl-paste").arg("--no-newline").output() {
            if output.status.success() {
                return String::from_utf8(output.stdout).ok();
            }
        }

        self.fallback
            .lock()
            .ok()
            .map(|value| value.clone())
            .filter(|value| !value.is_empty())
    }
}
