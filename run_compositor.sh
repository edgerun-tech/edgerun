#!/bin/bash
# Run the edgerun compositor

set -e

COMPOSITOR="./target/release/edgerun-compositor"
WAYLAND_SOCKET="/tmp/edgerun-wayland-0"

# Kill any existing instances
pkill -f edgerun-compositor 2>/dev/null || true
rm -f "$WAYLAND_SOCKET" 2>/dev/null || sudo rm -f "$WAYLAND_SOCKET" 2>/dev/null || true

echo "Starting edgerun compositor..."
echo "Wayland socket: $WAYLAND_SOCKET"
echo ""
echo "The compositor will take over the display."
echo "To launch clients, use: WAYLAND_DISPLAY=$WAYLAND_SOCKET <command>"
echo ""
echo "Examples:"
echo "  WAYLAND_DISPLAY=$WAYLAND_SOCKET firefox"
echo "  WAYLAND_DISPLAY=$WAYLAND_SOCKET weston-flower"
echo "  WAYLAND_DISPLAY=$WAYLAND_SOCKET epiphany"
echo ""

# Run the compositor
exec $COMPOSITOR
