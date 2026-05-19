Name:           edex-de
Version:        1.0.0
Release:        1%{?dist}
Summary:        eDEX-DE — A sci-fi Wayland desktop environment
License:        GPL-3.0
URL:            https://github.com/eDEX-OS/eDEX-DE
Source0:        https://github.com/eDEX-OS/eDEX-DE/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust cargo pkg-config
BuildRequires:  libxkbcommon-devel libinput-devel libseat-devel
BuildRequires:  libdrm-devel mesa-libgbm-devel wayland-devel
BuildRequires:  vulkan-headers pixman-devel systemd-devel

Requires:       libxkbcommon libinput libseat libdrm mesa-libgbm
Requires:       wayland-libs vulkan-loader systemd-libs dbus
Recommends:     xdg-desktop-portal

%description
A Wayland compositor and desktop environment inspired by eDEX-UI.
Built in pure Rust using smithay and wgpu. Features a sci-fi terminal
interface, tiling window management, app launcher, and system dashboard.

%prep
%autosetup -n eDEX-DE-%{version}

%build
cargo build --release --locked -p edex-de

%install
install -Dm755 target/release/edex-de \
    %{buildroot}%{_bindir}/edex-de

install -Dm644 packaging/session/edex-de.desktop \
    %{buildroot}%{_datadir}/wayland-sessions/edex-de.desktop

install -Dm755 packaging/session/edex-de-startup.sh \
    %{buildroot}%{_libdir}/edex-de/edex-de-startup.sh

install -Dm644 packaging/session/edex-de-portals.conf \
    %{buildroot}%{_datadir}/xdg-desktop-portal/edex-de-portals.conf

install -dm755 %{buildroot}%{_datadir}/edex-de/themes
install -Dm644 themes/*.toml \
    %{buildroot}%{_datadir}/edex-de/themes/

%files
%license LICENSE
%doc README.md
%{_bindir}/edex-de
%{_datadir}/wayland-sessions/edex-de.desktop
%{_libdir}/edex-de/edex-de-startup.sh
%{_datadir}/xdg-desktop-portal/edex-de-portals.conf
%{_datadir}/edex-de/themes/

%changelog
* Mon May 19 2026 eDEX-OS <edex-de@github.com> - 1.0.0-1
- Initial RPM release
