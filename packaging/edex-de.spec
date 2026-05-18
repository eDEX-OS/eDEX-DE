Name:           edex-de
Version:        0.13.0
Release:        1%{?dist}
Summary:        Sci-fi themed Wayland Desktop Environment for Hyprland

License:        GPL-3.0
URL:            https://github.com/eDEX-OS/eDEX-DE
Source0:        https://github.com/eDEX-OS/eDEX-DE/archive/refs/tags/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  rust cargo nodejs npm pkg-config openssl-devel
BuildRequires:  webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel

Requires:       webkit2gtk4.1 gtk3 libappindicator-gtk3
Recommends:     pipewire-pulse NetworkManager fprintd systemd

%description
eDEX-DE is a sci-fi themed Wayland desktop environment built on Tauri v2
and Hyprland. Features a terminal emulator, file manager, system monitor,
app launcher, NetworkManager integration, systemd service manager, and
PipeWire/PulseAudio audio control.

%prep
%autosetup -n eDEX-DE-%{version}

%build
npm install
npm run tauri -- build --no-bundle

%install
install -Dm755 src-tauri/target/release/edex-de %{buildroot}%{_bindir}/edex-de
install -Dm644 packaging/edex-de.desktop %{buildroot}%{_datadir}/applications/edex-de.desktop
install -Dm644 src-tauri/icons/128x128.png %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/edex-de.png
install -Dm644 src-tauri/icons/32x32.png %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/edex-de.png

%post
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database %{_datadir}/applications >/dev/null 2>&1 || :
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q %{_datadir}/icons/hicolor >/dev/null 2>&1 || :
fi

%files
%license LICENSE
%doc README.md
%{_bindir}/edex-de
%{_datadir}/applications/edex-de.desktop
%{_datadir}/icons/hicolor/128x128/apps/edex-de.png
%{_datadir}/icons/hicolor/32x32/apps/edex-de.png

%changelog
* 2024-01-01 eDEX-DE Contributors <> - 0.8.0-1
- Add Linux packaging manifests for Debian, Fedora, and Arch Linux
