# eDEX-DE Packaging

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/eDEX-OS/eDEX-DE/master/packaging/install.sh | bash
```

## Manual Install

### Arch Linux (AUR)
```bash
# Using yay
yay -S edex-de

# Manual PKGBUILD
git clone https://github.com/eDEX-OS/eDEX-DE.git
cd eDEX-DE/packaging
makepkg -si
```

### Debian / Ubuntu
```bash
# Download the latest .deb from GitHub Releases:
wget https://github.com/eDEX-OS/eDEX-DE/releases/latest/download/edex-de_VERSION_amd64.deb
sudo apt install ./edex-de_VERSION_amd64.deb
```

### Fedora / RHEL
```bash
sudo dnf install https://github.com/eDEX-OS/eDEX-DE/releases/latest/download/edex-de-VERSION-1.x86_64.rpm
```

## Runtime Dependencies

| Dependency | Purpose | Required? |
|---|---|---|
| `webkit2gtk-4.1` | Tauri WebView | ✅ Required |
| `gtk3` | Window manager | ✅ Required |
| `hyprland` or `sway` | Wayland compositor | ✅ Required |
| `pipewire-pulse` / `pulseaudio` | Audio control | Optional |
| `network-manager` | Network panel | Optional |
| `fprintd` | Fingerprint auth | Optional |
| `systemd` | Service manager | Optional |
| `pactl` / `wpctl` | Volume control CLI | Optional |
| `nmcli` | NetworkManager CLI | Optional |

## Build Dependencies

```bash
# Debian/Ubuntu
sudo apt install rustup nodejs npm pkg-config libssl-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev

# Fedora
sudo dnf install rust cargo nodejs npm pkg-config openssl-devel \
    webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel

# Arch
sudo pacman -S rust nodejs npm pkg-config openssl webkit2gtk-4.1 gtk3 libayatana-appindicator
```

## Building from Source

```bash
git clone https://github.com/eDEX-OS/eDEX-DE.git
cd eDEX-DE
npm install
npm run tauri -- build
```

Packages will be in `src-tauri/target/release/bundle/`.
