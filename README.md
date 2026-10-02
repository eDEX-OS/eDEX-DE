# eDEX-DE

[![CI](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml/badge.svg)](https://github.com/eDEX-OS/eDEX-DE/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/eDEX-OS/eDEX-DE?include_prereleases)](https://github.com/eDEX-OS/eDEX-DE/releases/latest)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-cyan.svg)](LICENSE)

**eDEX-DE** is a sci-fi desktop shell for [Hyprland](https://hyprland.org), written in Rust and drawn with
wgpu, in the style of [eDEX-UI](https://github.com/GitSquared/edex-ui). Hyprland tiles your applications;
eDEX-DE draws everything around them: the terminal, file browser, system dashboard, on-screen keyboard,
status bars, launcher, settings, privacy panel, notifications, power menu and the login screen.

It is the desktop of [eDEX-OS](https://github.com/eDEX-OS/eDEX-OS) and installs on any Arch-based system
with Hyprland 0.55 or newer.

## What you get

| Part | Implementation |
|---|---|
| Layout | Layer-shell canvas per output plus four invisible reserver surfaces, so Hyprland tiles app windows exactly into the terminal slot while the panels stay visible |
| Terminal | Multi-tab terminal on `alacritty_terminal` (alt screen, scroll regions, mouse reporting, bracketed paste, OSC 52, selection, scrollback) |
| Files | Clickable file browser with breadcrumbs, dotfiles toggle, open-in-terminal, `xdg-open` |
| Dashboard | CPU per core, memory, network sparklines, disks, processes; privacy indicators for Tor, Tailscale, VPN, WireGuard, fprintd, microphone and camera |
| Keyboard | Optional on-screen hex keyboard for touchscreens (Settings → Appearance, or `Ctrl+Shift+K`); off by default |
| Launcher | Fuzzy search over XDG desktop entries with launch history (tap `SUPER`, or `SUPER+Space`) |
| Settings | 14 categories wired to real backends: appearance, display, input, audio (wpctl), network (nmcli), bluetooth (bluetoothctl), power (upower/logind/power-profiles), security (hyprlock, fprintd), users, notifications, services (systemd), window manager, terminal, about |
| Privacy | Tor modes (off / socks5 / transparent), bootstrap and circuit state, NEWNYM, bridges; Tailscale login, exit nodes, peers; NetworkManager VPNs; DNS and firewall status |
| Notifications | eDEX-DE is the `org.freedesktop.Notifications` server: toasts, actions, history, do-not-disturb, per-app mute |
| Power / OSD | Lock, log out, suspend, hibernate, reboot, power off via logind; volume and brightness OSD |
| Greeter | `edex-greeter` for greetd (runs under `cage`), with fingerprint-aware PAM conversation, session picker and power buttons |
| Config | `~/.config/edex-de/config.toml` with live reload; Hyprland settings are exported to `~/.config/edex-de/hypr/generated.lua` |
| Themes | Tron, Matrix, Amber, Cyborg, Blade, Apollo, Interstellar, Horizon, Navy, Nord, Red, Purple; add your own in `~/.config/edex-de/themes` |
| IPC | `edex-de ipc …` drives the shell from Hyprland binds and scripts |

## Install

### Arch Linux / CachyOS

```bash
yay -S edex-de            # AUR
# or from a checkout:
scripts/build-pkg.sh && sudo pacman -U packaging/aur/edex-de-*.pkg.tar.zst
```

Then either pick **eDEX-DE** in your display manager, or use the eDEX greeter:

```bash
sudo cp /usr/share/edex-de/greetd/config.toml /etc/greetd/config.toml
sudo systemctl enable --now greetd.service
```

### Debian / Ubuntu and Fedora

`.deb` and `.rpm` packages are attached to every release. They are built and installed in CI but not
integration-tested on those distributions; Hyprland 0.55+ with Lua configuration must come from your
distribution or from source.

### From source

```bash
sudo pacman -S --needed rust hyprland cage greetd libxkbcommon wayland vulkan-icd-loader \
    pipewire wireplumber networkmanager bluez-utils upower brightnessctl hyprlock hypridle \
    hyprpolkitagent hyprsunset xdg-desktop-portal-hyprland xdg-desktop-portal-gtk ttf-jetbrains-mono-nerd \
    kitty wl-clipboard cliphist grim slurp playerctl librsvg
cargo build --release --locked --workspace --bins
```

For development, run the shell inside a nested Hyprland (or any wlr-layer-shell compositor):

```bash
EDEX_SHARE_DIR=$PWD/share cargo run -p edex-de -- run            # on Hyprland
EDEX_SHARE_DIR=$PWD/share cargo run -p edex-de -- run --no-hypr  # on sway etc.
cargo run -p edex-greeter -- --demo                              # greeter without greetd
```

## How a session starts

```
greetd (tty1) → edex-greeter-session (cage -s -- edex-greeter, text-login fallback) → edex-session → start-hyprland
   Hyprland reads ~/.config/hypr/hyprland.lua
      → require("edex") → /usr/share/edex-de/hypr/edex/*.lua (env, monitors, look, input, rules, binds, autostart)
      → dofile ~/.config/edex-de/hypr/generated.lua   (written by the settings panel)
      → dofile ~/.config/hypr/user.lua                (yours)
   hyprland.start → portals, hyprpolkitagent, hypridle, cliphist, systemctl --user start edex-de.service
```

`edex-de.service` is a user unit with `Restart=on-failure`, so a shell crash never ends the session.

## Windows and the centre tab strip

Applications tile into the centre panel, below its tab strip, which always stays visible. The strip
holds the terminal tabs, a tab for every window on the current workspace and one for every
minimized window. The focused window gets three controls at the right end of the strip:

* **↓** minimizes it into a tab (click the tab to bring it back),
* **□** maximizes it: it takes the full width while the side panels step aside; click again to
  restore,
* **×** closes it.

Clicking a terminal tab (or `+`) while apps cover the terminal minimizes them into tabs and shows
the terminal. Middle-click a window tab to close it.

## Keyboard shortcuts

Hyprland binds (from `share/hypr/edex/binds.lua`, editable in `~/.config/hypr/user.lua`):

| Keys | Action |
|---|---|
| `SUPER` (tap) or `SUPER+Space` | Launcher |
| `SUPER+,` | Settings |
| `SUPER+P` | Privacy panel |
| `SUPER+N` | Notification history |
| `SUPER+Escape` | Power menu |
| `SUPER+Return` / `SUPER+F1` | Focus the eDEX terminal / file panel |
| `SUPER+Shift+Return` | kitty |
| `SUPER+Q`, `SUPER+V` | Close, float the focused window |
| `SUPER+M` | Minimize the focused window into a tab in the centre panel |
| `SUPER+E` | Files: ranger in a new terminal tab, in the file panel's directory |
| `SUPER+F` | Maximize: full width, side panels hidden, top bar and window controls stay |
| `SUPER+SHIFT+F` | True fullscreen over everything |
| `SUPER+CTRL+F` | Hide / show the side panels for all apps |
| `SUPER+H/J/K/L`, `SUPER+Shift+…`, `SUPER+Ctrl+…` | Focus, move, resize |
| `SUPER+1..0`, `SUPER+Shift+1..0` | Switch / move to workspace |
| `SUPER+S` | Scratchpad |
| `SUPER+L` | Lock (hyprlock) |
| `Print`, `SUPER+Shift+S` | Screenshot (grim + slurp) |
| Media keys | Volume, brightness and playback through `edex-de ipc` / playerctl |

Inside the shell:

| Keys | Action |
|---|---|
| `Ctrl+Shift+T` / `Ctrl+Shift+W` | New / close terminal tab |
| `Alt+1..9`, `Ctrl+Tab` | Switch tab |
| `Ctrl+Shift+C` / `Ctrl+Shift+V` | Copy / paste |
| `Shift+PgUp` / `Shift+PgDn` | Scrollback |
| `Ctrl+Shift+F` | Toggle focus between terminal and file panel |
| `Ctrl+Shift+K` | Show / hide the on-screen keyboard |
| Overlays: `Esc` closes, `Tab`/arrows move, `Enter` activates, `Ctrl+PgUp/PgDn` switch tabs |

## Configuration

`~/.config/edex-de/config.toml` is created on first start and reloaded live when edited. Every key is
optional:

```toml
[appearance]
theme = "tron"            # any file in /usr/share/edex-de/themes or ~/.config/edex-de/themes
font = "JetBrainsMono Nerd Font"
font_size = 14.0
border_glow = 0.8
scanlines = true
animations = true
keyboard_visible = false  # on-screen hex keyboard, for touchscreens
boot_animation = true

[layout]
fs_split = 0.20           # file panel width
sysinfo_split = 0.78      # where the system panel starts
reserve_side_panels = true

[terminal]
shell = ""                # empty = your login shell
scrollback = 10000
font_size = 13.0
cursor = "block"          # block | underline | beam
cursor_blink = true
bell = "visual"           # visual | audible | none
osc52_read = false

[launcher]
terminal_command = "kitty -e"

[notifications]
dnd = false
timeout_ms = 5000
max_visible = 4
muted_apps = []

[wm]                      # exported to ~/.config/edex-de/hypr/generated.lua
gaps_in = 4
gaps_out = 8
border = 2
layout = "dwindle"        # dwindle | master | scrolling
workspaces = 9
animations = true
blur = false
rounding = 0

[input]
kb_layout = "us"
kb_variant = ""
kb_options = ""
repeat_rate = 30
repeat_delay = 300
natural_scroll = true
tap_to_click = true
sensitivity = 0.0

[display]
night_light = false
night_temp = 4000
[[display.monitors]]
name = ""                 # empty = every output
mode = "preferred"
position = "auto"
scale = 1.0

[power]                   # exported to hypridle.conf
dim_after = 300
lock_after = 600
dpms_after = 900
suspend_after = 0
profile = "balanced"
lid_close = "suspend"
lock_on_sleep = true

[privacy]
tor_mode_on_login = false
tailscale_exit_node = ""
fingerprint_login = true
```

The settings panel edits this file; `edex-de ipc reload` re-reads it and regenerates the Hyprland side.

## IPC

```bash
edex-de ipc toggle launcher|settings|privacy|notifications|power
edex-de ipc focus terminal|filesystem
edex-de ipc audio volume +5 | volume 40 | mute | mic-mute
edex-de ipc brightness -10
edex-de ipc theme matrix
edex-de ipc notify "Summary" "Body"
edex-de ipc state            # JSON: outputs, frames, overlay, terminal, hyprland, status
edex-de ipc reload | quit
```

See [docs/ipc.md](docs/ipc.md) for the wire format.

## Privacy panel semantics

* **Tor off** — nothing is routed through Tor.
* **socks5** — tor runs; applications configured for `127.0.0.1:9050` use it.
* **transparent** — on eDEX-OS, `edex-tor-mode transparent` redirects all TCP and DNS through Tor with a
  fail-closed nftables policy (LAN and Tailscale excepted). The panel asks for confirmation first.
* Tailscale actions call the `tailscale` CLI; eDEX-OS grants your user operator rights so no password is needed.
* Indicators come from `/run/edex-tor-mode`, listening sockets, interface state and `/proc/*/fd` scans
  (microphone and camera use).

## Themes

Themes are TOML files (see `themes/tron.toml` for the full schema). Switch live with
`edex-de ipc theme <name>` or from Settings → Appearance.

## Testing

* `cargo test --workspace` — unit tests including a real PTY, a private D-Bus notification round-trip,
  layout snapshots and parsers for every CLI backend.
* `scripts/smoke-sway.sh` — headless sway + llvmpipe: renders the shell, sends a notification, tiles a
  real window into the terminal slot, opens every overlay and stores screenshots under `target/smoke/`.
* `scripts/smoke-greeter.sh` — renders the greeter in demo mode.
* `scripts/build-pkg.sh` — builds the Arch package from the checkout (CI runs it in an Arch container).

See [docs/testing.md](docs/testing.md).

## Troubleshooting

* **Nothing but Hyprland's default look after login** — the shell is a user service: check
  `systemctl --user status edex-de` and `journalctl --user -u edex-de`. The session log is in
  `~/.local/state/edex-de/session.log`.
* **GPU errors** — eDEX-DE uses Vulkan and falls back to GL. `edex-de ipc state` shows the adapter; on VMs
  install `vulkan-swrast` (llvmpipe).
* **Hyprland config errors** — `~/.config/hypr/hyprland.lua` must `require` the system file; run
  `Hyprland --verify-config` after editing `user.lua`.
* **Another notification daemon** — if dunst or mako own `org.freedesktop.Notifications`, toasts are
  disabled; remove the other daemon.
* **Greeter** — `journalctl -u greetd`; run `edex-greeter --demo` inside a session to test the UI.

## License

GPL-3.0. Inspired by eDEX-UI by GitSquared.
