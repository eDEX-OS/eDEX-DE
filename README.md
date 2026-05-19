# eDEX-DE

[![CI](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml/badge.svg)](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/eDEX-OS/eDEX-DE)](https://github.com/eDEX-OS/eDEX-DE/releases/latest)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-cyan.svg)](LICENSE)

**eDEX-DE** is a full Wayland desktop environment built from scratch in pure Rust, faithful to the
sci-fi aesthetic of [eDEX-UI](https://github.com/GitSquared/edex-ui). It is a complete DE — compositor,
tiling WM, terminal, launcher, settings, and system dashboard — with no dependency on Hyprland, X11,
or any other compositor.

---

## Features

| | |
|---|---|
| 🖥️ **Full Wayland compositor** | DRM/KMS, libinput, XDG-shell via [smithay](https://github.com/Smithay/smithay) |
| 🎨 **eDEX-UI aesthetic** | 3-panel sci-fi layout, glowing cyan borders, animated hex keyboard |
| 🪟 **Tiling window manager** | 0 apps → eDEX fullscreen · 1 app → fullscreen · 2+ → equal columns |
| 💻 **Built-in terminal** | PTY-backed emulator, multi-tab, ANSI/VT100, clipboard support |
| 📂 **Filesystem panel** | Browse and navigate the filesystem from within eDEX |
| 📊 **System dashboard** | Live CPU, RAM, network, disk I/O, and process list |
| 🚀 **App launcher** | `Alt+Space` fuzzy launcher scanning all XDG `.desktop` files |
| ⚙️ **Settings panel** | 14-category settings UI — `Super+,` to open |
| 🔒 **Privacy indicators** | Live status for Tailscale, WireGuard, Tor, fprintd, mic, camera |
| 🎨 **Themes** | Tron (default), Matrix, Amber — TOML-configurable |
| 📦 **Package support** | Arch (AUR), Debian/Ubuntu (`.deb`), Fedora/RHEL (`.rpm`) |

---

## Installation

### Arch Linux (AUR)

```bash
yay -S edex-de
```

Log out and select **eDEX-DE** from your display manager (GDM, SDDM, ly, etc.).

### Debian / Ubuntu

```bash
# Download from https://github.com/eDEX-OS/eDEX-DE/releases/latest
sudo dpkg -i edex-de-*-amd64.deb
```

### Fedora / RHEL

```bash
# Download from https://github.com/eDEX-OS/eDEX-DE/releases/latest
sudo rpm -i edex-de-*.x86_64.rpm
```

### From Source

**Build dependencies:**

```bash
# Debian / Ubuntu
sudo apt-get install -y \
  libdrm-dev libgbm-dev libinput-dev libseat-dev \
  libxkbcommon-dev libwayland-dev libvulkan-dev \
  libpixman-1-dev libudev-dev libsystemd-dev

# Fedora
sudo dnf install -y \
  libdrm-devel mesa-libgbm-devel libinput-devel libseat-devel \
  libxkbcommon-devel wayland-devel vulkan-loader-devel \
  pixman-devel systemd-devel

# Arch
sudo pacman -S --needed \
  libdrm mesa libinput seatd libxkbcommon wayland vulkan-icd-loader \
  pixman systemd
```

**Build and install:**

```bash
git clone https://github.com/eDEX-OS/eDEX-DE.git
cd eDEX-DE
cargo build --release -p edex-de

sudo install -Dm755 target/release/edex-de             /usr/bin/edex-de
sudo install -Dm644 packaging/session/edex-de.desktop  /usr/share/wayland-sessions/edex-de.desktop
sudo install -Dm755 packaging/session/edex-de-startup.sh /usr/lib/edex-de/edex-de-startup.sh
sudo install -Dm644 packaging/session/edex-de-portals.conf /usr/share/xdg-desktop-portal/edex-de-portals.conf
sudo install -Dm644 themes/*.toml                      -t /usr/share/edex-de/themes/
```

Log out and select **eDEX-DE** from your display manager.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Alt+Space` | Open app launcher |
| `Alt+Tab` | Cycle window focus |
| `Super+Q` | Close focused window |
| `Super+1` – `Super+9` | Switch workspace |
| `Super+,` | Open settings panel |
| `Super+L` | Lock screen |

---

## Themes

Three built-in themes live in `themes/` as TOML files:

| Theme | Style |
|---|---|
| **tron** *(default)* | Cyan on near-black — classic eDEX-UI look |
| **matrix** | Bright green on black — terminal hacker aesthetic |
| **amber** | Warm amber on dark — retro CRT monitor feel |

The active theme is set in `~/.config/edex-de/config.toml`:

```toml
[appearance]
theme = "tron"   # tron | matrix | amber
```

---

## Architecture

```
edex-de/        — Binary entry point; wires compositor + renderer together
compositor/     — smithay DRM/KMS compositor, tiling WM, key dispatch
renderer/       — wgpu GPU renderer; layer-shell Wayland client
terminal/       — PTY + VTE terminal emulator (portable-pty, vte)
launcher/       — XDG .desktop fuzzy app launcher
settings/       — TOML config read/write (serde)
sysmon/         — sysinfo stats + privacy status (Tailscale, WireGuard, Tor …)
notifications/  — In-process notification store (no D-Bus required)
themes/         — TOML theme definitions
packaging/      — AUR PKGBUILD, Debian control, RPM spec, session files
website/        — GitHub Pages landing site
```

### How the compositor and renderer interact

The compositor (smithay) starts first, binding a Wayland socket. The renderer then
connects as a Wayland client and renders the eDEX UI as a `zwlr_layer_surface_v1`
at `Layer::Background`. All client windows (XDG-shell surfaces) appear above it.
Shared state (launcher open, settings open, lock screen) is exchanged via
`Arc<Mutex<bool>>` flags passed between threads.

---

## Configuration

Config is stored at `~/.config/edex-de/config.toml` and is created with defaults
on first launch.

```toml
[appearance]
theme = "tron"        # tron | matrix | amber
border_glow = 0.8     # 0.0–1.0
font_size = 14        # pt

[terminal]
shell = "/bin/bash"
scrollback = 10000
font_size = 13

[compositor]
gaps = 4              # px between tiled windows
workspaces = 9
```

---

## Privacy Indicators

The status bar shows live indicators for privacy-sensitive services:

| Indicator | What it checks |
|---|---|
| **VPN** | Tailscale (`tailscale status`) |
| **WireGuard** | `/proc/net/dev` + `ip link` for `wg*` interfaces |
| **Tor** | `systemctl is-active tor` |
| **Fingerprint** | `systemctl is-active fprintd` |
| **Microphone** | `/proc/*/fd` symlinks to `/dev/snd/` capture devices |
| **Camera** | `/proc/*/fd` symlinks to `/dev/video*` |

---

## Building & Development

```bash
# Check everything compiles
cargo check --workspace

# Lint (zero warnings policy)
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Run tests
cargo test --workspace

# Build release binary
cargo build --release -p edex-de
```

CI runs `cargo check`, `cargo clippy`, `cargo fmt --check`, and `cargo test` on
every push to `master` and every pull request.

---

## Troubleshooting

### Black screen / blinking cursor after login

This means the compositor failed to acquire seat access (DRM/KMS devices).

**Fix — add yourself to the required groups:**

```bash
sudo usermod -aG seat,input,video $USER
```

Then **log out completely** (full logout, not just switching TTYs) and log back in.
The new group membership only takes effect in a fresh login session.

**Check the session log** to see the exact error:

```bash
cat ~/.local/share/edex-de/session.log
```

**Verify systemd-logind is running** (required for seat management):

```bash
systemctl status systemd-logind
```

**Verify DRM devices exist:**

```bash
ls -la /dev/dri/
```

### Display manager

eDEX-DE works with any display manager that supports Wayland sessions:
SDDM, GDM, LightDM (with lightdm-gtk-greeter ≥ 2.0), ly, or greetd.
**KDE's SDDM is fully supported** — it is the recommended DM.

---



Releases are triggered by pushing a version tag:

```bash
git tag v2.1.0
git push origin --tags
```

The release workflow automatically builds `.deb`, `.rpm`, and `.tar.gz` artifacts
and publishes a GitHub Release. Tags containing `-beta`, `-rc`, or `-alpha` are
marked as pre-releases.

---

## License

GPL-3.0 — see [LICENSE](LICENSE).

Inspired by [eDEX-UI](https://github.com/GitSquared/edex-ui) by [@GitSquared](https://github.com/GitSquared).
