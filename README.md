# eDEX-DE

[![CI](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml/badge.svg)](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/eDEX-OS/eDEX-DE)](https://github.com/eDEX-OS/eDEX-DE/releases/latest)

A sci-fi Wayland desktop environment inspired by [eDEX-UI](https://github.com/GitSquared/edex-ui).

Built from scratch in pure Rust using:
- **[smithay](https://github.com/Smithay/smithay)** — Wayland compositor framework (DRM/KMS, libinput, XDG-shell)
- **[wgpu](https://github.com/gfx-rs/wgpu)** — GPU-accelerated UI rendering (Vulkan backend, WGSL shaders)
- **[glyphon](https://github.com/grovesNL/glyphon)** — wgpu text rendering
- **[portable-pty](https://github.com/wez/wezterm/tree/main/pty)** — terminal emulation
- **[sysinfo](https://github.com/GuillaumeGomez/sysinfo)** — system statistics

## Features

- 🖥️ Full Wayland compositor (no Hyprland, no X11 required)
- 🎨 Faithful eDEX-UI aesthetic — 3-panel sci-fi layout, glowing cyan borders, hex keyboard
- 💻 Built-in terminal emulator with multi-tab support
- 📂 Filesystem browser panel
- 📊 Live system info panel (CPU, RAM, network, processes)
- 🚀 App launcher (Alt+Space or Meta key)
- ⚙️ Full settings panel — 14 categories (Super+Comma)
- 🔒 Privacy indicators: Tailscale, WireGuard, Tor, fprintd, mic/camera
- 🪟 Tiling window manager (0 windows → eDEX fullscreen; 1 → fullscreen; 2+ → columns)
- 🎨 Themes: Tron (default), Matrix, Amber

## Installation

### Arch Linux (AUR)
```bash
yay -S edex-de
```

### Debian / Ubuntu
Download the `.deb` from the [latest release](https://github.com/eDEX-OS/eDEX-DE/releases/latest):
```bash
sudo dpkg -i edex-de-*-amd64.deb
```

### Fedora / RHEL
Download the `.rpm` from the [latest release](https://github.com/eDEX-OS/eDEX-DE/releases/latest):
```bash
sudo rpm -i edex-de-*.x86_64.rpm
```

### From source
```bash
# Install build deps (Ubuntu/Debian)
sudo apt-get install -y libdrm-dev libgbm-dev libinput-dev libseat-dev \
  libxkbcommon-dev libwayland-dev libvulkan-dev libpixman-1-dev libudev-dev

cargo build --release -p edex-de
sudo install -Dm755 target/release/edex-de /usr/bin/edex-de
sudo install -Dm644 packaging/session/edex-de.desktop /usr/share/wayland-sessions/edex-de.desktop
sudo install -Dm755 packaging/session/edex-de-startup.sh /usr/lib/edex-de/edex-de-startup.sh
```

Then log out and select **eDEX-DE** from your display manager.

## Architecture

```
edex-de/        — binary entry point
compositor/     — smithay DRM/KMS Wayland compositor + tiling WM
renderer/       — wgpu GPU renderer (layer-shell Wayland client)
terminal/       — PTY + VTE terminal emulator
launcher/       — XDG .desktop app launcher (Alt+Space)
settings/       — TOML config persistence
sysmon/         — system stats + privacy status checks
notifications/  — in-process notification store
themes/         — TOML theme files (tron, matrix, amber)
packaging/      — AUR PKGBUILD, .deb, .rpm, session files
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+Space` | App launcher |
| `Alt+Tab` | Cycle focus |
| `Super+Q` | Close focused window |
| `Super+1–9` | Switch workspace |
| `Super+Comma` | Open settings |
| `Super+L` | Lock screen |

## Themes

Three built-in themes in `themes/`:
- **Tron** (default) — cyan on dark
- **Matrix** — green terminal aesthetic  
- **Amber** — warm amber retro terminal

## License

GPL-3.0 — see [LICENSE](LICENSE)
