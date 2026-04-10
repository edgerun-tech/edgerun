#!/bin/bash
# Launch a Wayland client on the running edgerun compositor

WAYLAND_SOCKET="/tmp/edgerun-wayland-0"

if [ ! -S "$WAYLAND_SOCKET" ]; then
    echo "Error: Wayland socket $WAYLAND_SOCKET not found!"
    echo "Is the edgerun compositor running?"
    exit 1
fi

if [ $# -eq 0 ]; then
    echo "Usage: $0 <command> [args...]"
    echo ""
    echo "Examples:"
    echo "  $0 firefox https://www.google.com"
    echo "  $0 weston-flower"
    echo "  $0 epiphany"
    echo "  $0 gtk4-demo"
    exit 1
fi

export WAYLAND_DISPLAY=$WAYLAND_SOCKET
echo "Launching: $@"
exec "$@"
