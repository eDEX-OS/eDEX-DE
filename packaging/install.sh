#!/usr/bin/env bash
# eDEX-DE Universal Install Helper
# Usage: curl -fsSL https://raw.githubusercontent.com/eDEX-OS/eDEX-DE/master/packaging/install.sh | bash
set -euo pipefail

REPO="eDEX-OS/eDEX-DE"
BIN_NAME="edex-de"

if [ -f /etc/arch-release ]; then
    DISTRO="arch"
elif [ -f /etc/debian_version ]; then
    DISTRO="debian"
elif [ -f /etc/fedora-release ]; then
    DISTRO="fedora"
elif [ -f /etc/redhat-release ]; then
    DISTRO="fedora"
else
    DISTRO="unknown"
fi

echo "Detected distro: $DISTRO"

LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
echo "Latest version: $LATEST"

case "$DISTRO" in
    arch)
        echo "Installing via PKGBUILD..."
        tmpdir=$(mktemp -d)
        curl -fsSL "https://raw.githubusercontent.com/${REPO}/master/packaging/PKGBUILD" -o "$tmpdir/PKGBUILD"
        cd "$tmpdir"
        sed -i "s/pkgver=.*/pkgver=${LATEST#v}/" PKGBUILD
        makepkg -si --noconfirm
        ;;
    debian)
        echo "Installing .deb package..."
        tmpdir=$(mktemp -d)
        curl -fsSL "https://github.com/${REPO}/releases/download/${LATEST}/edex-de_${LATEST#v}_amd64.deb" \
             -o "$tmpdir/edex-de.deb"
        sudo apt install -y "$tmpdir/edex-de.deb"
        ;;
    fedora)
        echo "Installing .rpm package..."
        tmpdir=$(mktemp -d)
        curl -fsSL "https://github.com/${REPO}/releases/download/${LATEST}/edex-de-${LATEST#v}-1.x86_64.rpm" \
             -o "$tmpdir/edex-de.rpm"
        sudo dnf install -y "$tmpdir/edex-de.rpm"
        ;;
    *)
        echo "Unknown distro. Please install manually from:"
        echo "https://github.com/${REPO}/releases/latest"
        exit 1
        ;;
esac

echo ""
echo "✓ eDEX-DE installed successfully!"
echo "Run: $BIN_NAME"
echo "Or add to your Hyprland config: exec-once = $BIN_NAME"
