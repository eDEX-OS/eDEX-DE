# eDEX-DE Dependencies

## Runtime Dependencies (Linux)
- `libwebkit2gtk-4.1-0` or `webkit2gtk-4.0` — WebView rendering engine
- `libgtk-3-0` — GTK3 windowing
- `libssl3` / `libssl1.1` — TLS/SSL
- `libasound2` — ALSA audio (for rodio/alsa-sys)
- `bash` — Default shell for terminal PTY
- `pactl` (pulseaudio-utils) or `wpctl` (wireplumber) — Audio control
- `nmcli` (network-manager) — Network management
- `fprintd` (optional) — Fingerprint authentication
- `systemctl` (systemd) — Service management
- `journalctl` (systemd) — Service logs

## Rust Crates (Build Dependencies)
- `tauri` v2 — App framework
- `portable-pty` — PTY/terminal spawning
- `sysinfo` — System information (CPU, RAM, disk, processes)
- `tokio` — Async runtime
- `tokio-tungstenite` — WebSocket server for terminal
- `serde` / `serde_json` — Serialization
- `dirs` — XDG directories
- `rodio` — Audio playback
- `reqwest` — HTTP client

## npm Packages
- `preact` — UI framework
- `vite` — Build tool
- `@tauri-apps/api` — Tauri JavaScript API
- `@tauri-apps/plugin-global-shortcut` — Global keyboard shortcuts
- `@xterm/xterm` — Terminal emulator component
- `@xterm/addon-fit` — Terminal auto-resize
- `@xterm/addon-web-links` — Clickable URLs in terminal

## Build Tools
- Rust stable toolchain (`rustup`)
- Node.js 20+ LTS
- `@tauri-apps/cli` — Tauri CLI (via npm)
