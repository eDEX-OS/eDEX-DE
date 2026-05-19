#!/bin/bash
# eDEX-DE session startup script
# All output is logged to ~/.local/share/edex-de/session.log

LOG_DIR="$HOME/.local/share/edex-de"
LOG_FILE="$LOG_DIR/session.log"
mkdir -p "$LOG_DIR"

exec > >(tee -a "$LOG_FILE") 2>&1

echo "========================================" 
echo "eDEX-DE session start: $(date)"
echo "========================================" 

# Ensure XDG_RUNTIME_DIR is set and exists
if [ -z "$XDG_RUNTIME_DIR" ]; then
    export XDG_RUNTIME_DIR="/run/user/$(id -u)"
    echo "XDG_RUNTIME_DIR was unset, using $XDG_RUNTIME_DIR"
fi
if [ ! -d "$XDG_RUNTIME_DIR" ]; then
    mkdir -p "$XDG_RUNTIME_DIR"
    chmod 0700 "$XDG_RUNTIME_DIR"
    echo "Created XDG_RUNTIME_DIR: $XDG_RUNTIME_DIR"
fi

# Wayland environment
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=eDEX-DE
export XDG_CURRENT_DESKTOP=eDEX-DE
export MOZ_ENABLE_WAYLAND=1
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export CLUTTER_BACKEND=wayland
export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE=1

echo "Groups: $(groups)"
echo "DBUS_SESSION_BUS_ADDRESS: ${DBUS_SESSION_BUS_ADDRESS:-<unset>}"
echo "XDG_RUNTIME_DIR: $XDG_RUNTIME_DIR"
echo "WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-<unset>}"

# Trap to log exit reason
_on_exit() {
    local code=$?
    echo ""
    echo "eDEX-DE exited at $(date) with code $code"
    if [ $code -ne 0 ]; then
        echo ""
        echo "=== SESSION FAILED ==="
        echo "Check $LOG_FILE for details."
        echo "Common fixes:"
        echo "  1. Add yourself to the seat group: sudo usermod -aG seat \$USER"
        echo "     Then LOG OUT and log back in (not just switch TTY)."
        echo "  2. Ensure systemd-logind is running: systemctl status systemd-logind"
        echo "  3. Ensure /dev/dri/card* exists: ls /dev/dri/"
        echo ""
        # Give user a chance to read the message on the TTY before it disappears
        if [ -t 1 ]; then
            read -r -t 30 -p "Press Enter or wait 30s to exit..." || true
        fi
    fi
}
trap _on_exit EXIT

# Check that the binary exists
if [ ! -x /usr/bin/edex-de ]; then
    echo "ERROR: /usr/bin/edex-de not found or not executable"
    exit 1
fi

# Check DRM devices are accessible
if ! ls /dev/dri/card* >/dev/null 2>&1; then
    echo "ERROR: No DRM devices found at /dev/dri/card*"
    exit 1
fi

# Check seat group membership (warn only — libseat may still work via logind)
if ! groups | grep -qE '\bseat\b'; then
    echo "WARNING: User $(whoami) is not in the 'seat' group."
    echo "  If startup fails, run: sudo usermod -aG seat $(whoami)"
    echo "  Then log out completely and log back in."
fi

# Launch — SDDM already provides a D-Bus session, so no dbus-run-session needed
echo "Launching /usr/bin/edex-de ..."
exec /usr/bin/edex-de "$@"
