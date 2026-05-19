use std::process::Command;

use anyhow::Result;

use crate::desktop::AppEntry;

pub fn launch_app(app: &AppEntry) -> Result<()> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&app.exec);
    cmd.env(
        "WAYLAND_DISPLAY",
        std::env::var("WAYLAND_DISPLAY").unwrap_or_default(),
    );
    cmd.env("DISPLAY", std::env::var("DISPLAY").unwrap_or_default());
    cmd.spawn()?;
    Ok(())
}
