#!/bin/bash
# eDEX-DE session startup — sets Wayland env vars and launches compositor
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=eDEX-DE
export XDG_CURRENT_DESKTOP=eDEX-DE
export MOZ_ENABLE_WAYLAND=1
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export CLUTTER_BACKEND=wayland

# Start D-Bus session if not already running
if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then
    exec dbus-run-session -- /usr/bin/edex-de "$@"
else
    exec /usr/bin/edex-de "$@"
fi
