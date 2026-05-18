<p align="center">
  <img alt="eDEX-DE" src="media/logo.png" width="200">
  <br><br>
  <a href="https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/eDEX-OS/eDEX-DE/releases/latest"><img alt="Latest Release" src="https://img.shields.io/github/v/release/eDEX-OS/eDEX-DE?style=flat"></a>
  <a href="https://github.com/eDEX-OS/eDEX-DE/blob/master/LICENSE"><img alt="License" src="https://img.shields.io/github/license/eDEX-OS/eDEX-DE?style=flat"></a>
  <a href="https://edex-os.github.io/eDEX-DE/"><img alt="Docs" src="https://img.shields.io/badge/docs-GitHub%20Pages-00e5ff?style=flat"></a>
</p>

# eDEX-DE

> A sci-fi desktop environment frontend for Hyprland — built in Rust + TypeScript/Preact with Tauri v2.

📖 **[Documentation](https://edex-os.github.io/eDEX-DE/)** · 🚀 **[Install Guide](https://edex-os.github.io/eDEX-DE/install/)** · 🌐 **[GitHub](https://github.com/eDEX-OS/eDEX-DE)**

---

## Highlights

- **Sci-fi terminal DE** — fullscreen tron-themed interface inspired by eDEX-UI, rebuilt from scratch
- **Tauri v2 + Rust backend** — blazing-fast, memory-safe; replaces Electron entirely
- **TypeScript + Preact UI** — typed, component-based frontend; no jQuery, no vanilla JS globals
- **Hyprland IPC integration** — live window list, workspace switcher, config generator, event streaming
- **Spotlight/Meta app launcher** — fuzzy-search over `.desktop` apps, shell commands, math eval; Alt+Space or Meta
- **System integrations** — PipeWire/PulseAudio volume, NetworkManager WiFi, fprintd fingerprint auth, systemd service manager with logs
- **Native packages** — `.deb` (Debian/Ubuntu), `.rpm` (Fedora), `PKGBUILD` (Arch/AUR), universal installer
- **Wayland-native** — fullscreen layer surface, no decorations, plays nicely with Hyprland window rules

---

## Repository Structure

```
eDEX-DE/
├── .github/workflows/      # CI, release, and GitHub Pages deploy workflows
├── src-tauri/              # Rust backend (Tauri v2)
│   ├── src/
│   │   ├── commands/       # All Tauri IPC commands (sysinfo, terminal, filesystem, audio, network, etc.)
│   │   ├── main.rs         # App entry, Tauri builder
│   │   └── lib.rs          # Plugin registration, global shortcuts, Hyprland event listener
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # App config (window, bundle targets, identifier)
├── ui/                     # TypeScript + Preact frontend
│   ├── src/
│   │   ├── components/     # All UI components (Terminal, FileSystem, SysInfo, Launcher, etc.)
│   │   ├── context/        # Settings and SysInfo Preact contexts
│   │   ├── hooks/          # Custom hooks
│   │   ├── ipc/            # Typed Tauri IPC wrappers
│   │   ├── styles/         # main.css — all styles
│   │   └── types/          # TypeScript interfaces/types
│   └── index.html
├── packaging/
│   ├── PKGBUILD            # Arch Linux AUR package script
│   ├── edex-de.spec        # RPM spec for Fedora/RHEL
│   ├── edex-de.desktop     # XDG desktop entry
│   └── install.sh          # Universal curl-pipe installer
├── docs/                   # GitHub Pages documentation site
└── Dependencies.md         # Runtime and build dependencies
```

---

## Getting Started

### Install from Package

See the **[Installation Guide](https://edex-os.github.io/eDEX-DE/install/)** for per-distro instructions, or use the universal installer:

```bash
curl -fsSL https://raw.githubusercontent.com/eDEX-OS/eDEX-DE/master/packaging/install.sh | bash
```

**Debian/Ubuntu:**
```bash
wget https://github.com/eDEX-OS/eDEX-DE/releases/latest/download/edex-de_amd64.deb
sudo dpkg -i edex-de_amd64.deb
```

**Fedora:**
```bash
sudo rpm -i https://github.com/eDEX-OS/eDEX-DE/releases/latest/download/edex-de.x86_64.rpm
```

**Arch Linux (AUR):**
```bash
yay -S edex-de
# or: paru -S edex-de
```

### Build from Source

#### Prerequisites

- Rust stable toolchain (`rustup` — https://rustup.rs)
- Node.js 20+ LTS
- System libraries (Debian/Ubuntu):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev                          libasound2-dev libayatana-appindicator3-dev pkg-config
```

#### Build

```bash
git clone https://github.com/eDEX-OS/eDEX-DE.git
cd eDEX-DE
npm install
npm run tauri -- build
```

#### Dev Mode

```bash
npm run tauri -- dev
```

---

## Hyprland Setup

Add to your `~/.config/hypr/hyprland.conf`:

```ini
# Launch eDEX-DE
exec-once = edex-de

# App Launcher keybind (Meta key)
bind = SUPER, Space, exec, edex-de --launcher

# Window rules
windowrule = pin, edex-de
windowrule = fullscreen, edex-de
windowrule = nodecor, edex-de
windowrule = noborder, edex-de
windowrule = noshadow, edex-de
windowrule = noanim, edex-de
```

Use the built-in **Hyprland Config Generator** (Ctrl+Shift+H → Save Config) to generate and save these rules automatically to `~/.config/edex-de/hyprland-integration.conf`.

---

## App Launcher

Open with **Alt+Space** (Spotlight style) or **Meta+Space** (configured via Hyprland bind above).

| Mode | Prefix | Example |
|------|--------|---------|
| App search | *(none)* | `firefox` |
| Shell command | `>` | `> htop` |
| Math | *(expression)* | `2 + 2 * 3` |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Alt+Space | Open App Launcher |
| Ctrl+Shift+S | Open Settings |
| Ctrl+Shift+H | Open Hyprland Config |

---

## Custom Rust Components

| Component | Description |
|-----------|-------------|
| `commands/sysinfo.rs` | CPU, RAM, disk, network, process list via `sysinfo` crate |
| `commands/terminal.rs` | PTY server using `portable-pty` + tokio WebSocket |
| `commands/filesystem.rs` | File browser with fuzzy search via `fuzzy-matcher` |
| `commands/audio_control.rs` | PipeWire/PulseAudio volume via `pactl`/`wpctl` CLI |
| `commands/network.rs` | NetworkManager via `nmcli` CLI |
| `commands/fingerprint.rs` | fprintd fingerprint auth via CLI |
| `commands/systemd.rs` | systemd service manager via `systemctl`/`journalctl` |
| `commands/hyprland.rs` | Hyprland IPC via Unix socket (raw, no crate) |
| `commands/launcher.rs` | .desktop file parser + fuzzy app search |

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

GPL-3.0 — see [LICENSE](LICENSE).

---

*Inspired by the legendary [eDEX-UI](https://github.com/GitSquared/edex-ui) by GitSquared.*
