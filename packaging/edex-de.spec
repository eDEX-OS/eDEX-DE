Name:           edex-de
Version:        1.2.0
Release:        1%{?dist}
Summary:        Sci-fi themed Wayland Desktop Environment built on Hyprland

License:        GPL-3.0
URL:            https://github.com/eDEX-OS/eDEX-DE
Source0:        https://github.com/eDEX-OS/eDEX-DE/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  rust cargo nodejs npm pkg-config openssl-devel
BuildRequires:  webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel

# Core runtime
Requires:       webkit2gtk4.1 gtk3 libappindicator-gtk3 openssl
# Wayland session
Requires:       hyprland xdg-desktop-portal xdg-desktop-portal-hyprland
# Audio
Requires:       pipewire wireplumber pipewire-pulse
# Session utilities
Requires:       wl-clipboard mako swayidle polkit-kde jq
# Optional integrations
Recommends:     NetworkManager fprintd bluez tailscale tor

%description
eDEX-DE is a sci-fi themed Wayland desktop environment built on Tauri v2
and Hyprland. Log in with "eDEX-DE" at the display manager to get the
full sci-fi shell experience. Features a terminal emulator, file manager,
system monitor, app launcher, NetworkManager integration, systemd service
manager, PipeWire/PulseAudio audio control, Tor/VPN/Tailscale privacy
panel, and Bluetooth support.

%prep
%autosetup -n eDEX-DE-%{version}

%build
npm install
npm run tauri -- build --no-bundle

%install
# Main binary
install -Dm755 target/release/edex-de %{buildroot}%{_bindir}/edex-de

# Session startup script
install -Dm755 packaging/edex-de-session %{buildroot}%{_bindir}/edex-de-session

# Wayland session entry (makes eDEX-DE appear in the login screen)
install -Dm644 packaging/edex-de-session.desktop \
    %{buildroot}%{_datadir}/wayland-sessions/edex-de.desktop

# Application entry
install -Dm644 packaging/edex-de.desktop \
    %{buildroot}%{_datadir}/applications/edex-de.desktop

# Bundled Hyprland config
install -Dm644 packaging/edex-de-hyprland.conf \
    %{buildroot}%{_sysconfdir}/xdg/edex-de/hyprland.conf

# Icons
install -Dm644 src-tauri/icons/128x128.png \
    %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/edex-de.png
install -Dm644 src-tauri/icons/32x32.png \
    %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/edex-de.png

%post
command -v update-desktop-database &>/dev/null && \
    update-desktop-database %{_datadir}/applications &>/dev/null || :
command -v gtk-update-icon-cache &>/dev/null && \
    gtk-update-icon-cache -q %{_datadir}/icons/hicolor &>/dev/null || :

%files
%license LICENSE
%{_bindir}/edex-de
%{_bindir}/edex-de-session
%{_datadir}/wayland-sessions/edex-de.desktop
%{_datadir}/applications/edex-de.desktop
%config(noreplace) %{_sysconfdir}/xdg/edex-de/hyprland.conf
%{_datadir}/icons/hicolor/128x128/apps/edex-de.png
%{_datadir}/icons/hicolor/32x32/apps/edex-de.png

%changelog
* 2025-05-18 eDEX-DE Contributors <> - 1.2.0-1
- Session infrastructure: Wayland session entry, startup script, bundled Hyprland config
- Comprehensive settings panel: 14 pages including display, Bluetooth, power, users, notifications
- Privacy/security consolidated into Settings panel
- eDEX-DE now auto-starts as inescapable shell on login

* 2025-05-18 eDEX-DE Contributors <> - 1.1.4-1
- Add session infrastructure: wayland session entry, startup script, bundled Hyprland config
- Prevent window close when running as DE session
- Switch reqwest to native-tls (eliminates ring build issues on Arch)
