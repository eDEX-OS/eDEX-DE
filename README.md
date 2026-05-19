# eDEX-DE

A sci-fi Wayland desktop environment inspired by [eDEX-UI](https://github.com/GitSquared/edex-ui).

Built from scratch in pure Rust using:
- **[smithay](https://github.com/Smithay/smithay)** — Wayland compositor framework (DRM/KMS, libinput, XDG-shell)
- **[wgpu](https://github.com/gfx-rs/wgpu)** — GPU-accelerated UI rendering (Vulkan backend, WGSL shaders)
- **[glyphon](https://github.com/grovesNL/glyphon)** — wgpu text rendering
- **[termwiz](https://github.com/wez/wezterm/tree/main/termwiz)** + **[portable-pty](https://github.com/wez/wezterm/tree/main/pty)** — terminal emulation
- **[sysinfo](https://github.com/GuillaumeGomez/sysinfo)** — system statistics

## Features (planned)

- 🖥️ Full Wayland compositor (no Hyprland, no X11 required)
- 🎨 Faithful eDEX-UI aesthetic — 3-panel sci-fi layout, glowing cyan borders, hex keyboard
- 💻 Built-in terminal emulator with multi-tab support
- 📂 Filesystem browser panel
- 📊 Live system info panel (CPU, RAM, network, processes)
- 🚀 App launcher (Alt+Space)
- ⚙️ Full settings panel (14 categories)
- 🔒 Privacy features: Tor, Tailscale, WireGuard VPN
- 🪟 Tiling window manager

## Installation

### Arch Linux (AUR)
```bash
yay -S edex-de
```

### From source
```bash
cargo build --release
sudo install -Dm755 target/release/edex-de /usr/bin/edex-de
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+Space` | App launcher |
| `Super+Q` | Close focused window |
| `Super+1-0` | Switch workspace |
| `Super+F` | Toggle fullscreen |
| `Super+L` | Lock screen |
| `Ctrl+Shift+S` | Open settings |

## License

GPL-3.0 — see [LICENSE](LICENSE)
