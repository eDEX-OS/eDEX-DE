# Fallbacks for rpm without systemd-rpm-macros (e.g. building on Debian/Ubuntu).
%{!?_userunitdir:%global _userunitdir %{_prefix}/lib/systemd/user}
%{!?_tmpfilesdir:%global _tmpfilesdir %{_prefix}/lib/tmpfiles.d}

Name:           edex-de
Version:        3.3.0
Release:        1%{?dist}
Summary:        eDEX-DE - sci-fi desktop shell for Hyprland
License:        GPL-3.0-only
URL:            https://github.com/eDEX-OS/eDEX-DE
Source0:        https://github.com/eDEX-OS/eDEX-DE/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.95 cargo pkgconfig librsvg2-tools
BuildRequires:  libxkbcommon-devel wayland-devel vulkan-headers dbus-devel
Requires:       hyprland >= 0.55 cage greetd libxkbcommon wayland-libs vulkan-loader dbus polkit
Requires:       xdg-desktop-portal-gtk pipewire wireplumber NetworkManager bluez upower brightnessctl
Requires:       jetbrains-mono-fonts kitty ranger wl-clipboard grim slurp playerctl libnotify
Recommends:     hyprlock hypridle cliphist

%description
A Rust + wgpu shell drawn on Hyprland's layer shell in the style of eDEX-UI:
terminal, file browser, system dashboard, on-screen keyboard, launcher,
settings, privacy panel, notification server and a greetd greeter.
The RPM is built by CI; eDEX-DE is developed and tested on Arch/CachyOS.

%prep
%autosetup -n eDEX-DE-%{version}

%build
cargo build --release --locked --workspace --bins
assets/make-assets.sh assets/generated

%check
cargo test --release --locked --workspace

%install
install -Dm755 target/release/edex-de %{buildroot}%{_bindir}/edex-de
install -Dm755 target/release/edex-greeter %{buildroot}%{_bindir}/edex-greeter
install -Dm755 packaging/session/edex-session %{buildroot}%{_bindir}/edex-session
install -Dm644 packaging/session/edex-de.desktop %{buildroot}%{_datadir}/wayland-sessions/edex-de.desktop
install -Dm644 -t %{buildroot}%{_datadir}/applications packaging/applications/*.desktop
install -Dm644 packaging/session/edex-de-portals.conf %{buildroot}%{_datadir}/xdg-desktop-portal/edex-de-portals.conf
install -Dm644 packaging/systemd/edex-de.service %{buildroot}%{_userunitdir}/edex-de.service
install -Dm644 packaging/tmpfiles/edex-greeter.conf %{buildroot}%{_tmpfilesdir}/edex-greeter.conf
install -Dm644 packaging/polkit/50-edex-greeter-power.rules %{buildroot}%{_datadir}/polkit-1/rules.d/50-edex-greeter-power.rules
install -Dm644 packaging/greeter/greeter.toml %{buildroot}%{_sysconfdir}/edex-greeter/greeter.toml
install -Dm644 packaging/greetd/config.toml %{buildroot}%{_datadir}/edex-de/greetd/config.toml
install -Dm644 share/skel/hyprland.lua %{buildroot}%{_sysconfdir}/skel/.config/hypr/hyprland.lua
install -dm755 %{buildroot}%{_datadir}/edex-de/themes %{buildroot}%{_datadir}/edex-de/hypr %{buildroot}%{_datadir}/edex-de/backgrounds
install -m644 themes/*.toml %{buildroot}%{_datadir}/edex-de/themes/
cp -r share/hypr/. %{buildroot}%{_datadir}/edex-de/hypr/
install -m644 assets/generated/backgrounds/*.png %{buildroot}%{_datadir}/edex-de/backgrounds/
for s in 32 48 64 128 256 512; do
  install -Dm644 assets/generated/icons/edex-de-$s.png %{buildroot}%{_datadir}/icons/hicolor/${s}x${s}/apps/edex-de.png
done
install -Dm644 assets/logo.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/edex-de.svg

%files
%license LICENSE
%doc README.md docs/architecture.md docs/ipc.md
%{_bindir}/edex-de
%{_bindir}/edex-greeter
%{_bindir}/edex-session
%{_datadir}/wayland-sessions/edex-de.desktop
%{_datadir}/applications/edex-settings.desktop
%{_datadir}/applications/edex-privacy.desktop
%{_datadir}/applications/edex-files.desktop
%{_datadir}/xdg-desktop-portal/edex-de-portals.conf
%{_userunitdir}/edex-de.service
%{_tmpfilesdir}/edex-greeter.conf
%{_datadir}/polkit-1/rules.d/50-edex-greeter-power.rules
%config(noreplace) %{_sysconfdir}/edex-greeter/greeter.toml
%{_sysconfdir}/skel/.config/hypr/hyprland.lua
%{_datadir}/edex-de/
%{_datadir}/icons/hicolor/*/apps/edex-de.*

%changelog
* Fri Oct 02 2026 eDEX-OS <edex-de@github.com> - 3.3.0-1
- GPU and temperature monitors, eDEX-UI memory grid, ranger file manager, login screen redesign

* Thu Oct 01 2026 eDEX-OS <edex-de@github.com> - 3.2.0-1
- Real privacy state, WireGuard tunnels, settings desktop entries, Qt/GTK/KDE theming, side-panel toggle

* Wed Sep 30 2026 eDEX-OS <edex-de@github.com> - 3.1.0-1
- Working keyboard and pointer input; on-screen keyboard opt-in; Hyprland 0.56 Lua dispatch

* Mon Sep 28 2026 eDEX-OS <edex-de@github.com> - 3.0.0-1
- Rewrite as a Rust shell on Hyprland; greetd greeter; settings and privacy panels
