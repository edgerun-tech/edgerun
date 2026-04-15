#!/bin/bash
# Install Edgerun VFS Daemon

set -e

BIN_NAME="daemon"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/edgerun-vfs"
SERVICE_DIR="/etc/systemd/system"
USER="${SUDO_USER:-$USER}"

echo "=== Edgerun VFS Daemon Installer ==="
echo

# Build release binary
echo "Building release binary..."
cargo build --release -p edgerun-vfs --bin daemon

# Find the binary (works with both musl and gnu targets)
BIN_PATH=$(find target -name "$BIN_NAME" -type f -executable | grep release | head -1)

if [ -z "$BIN_PATH" ]; then
    echo "Error: Could not find built binary"
    exit 1
fi

echo "Found binary: $BIN_PATH"

# Install binary
echo "Installing binary to $INSTALL_DIR..."
sudo cp "$BIN_PATH" "$INSTALL_DIR/edgerun-vfs-daemon"
sudo chmod +x "$INSTALL_DIR/edgerun-vfs-daemon"

# Create config directory
echo "Creating config directory..."
sudo mkdir -p "$CONFIG_DIR"

# Install config if not exists
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Installing default config..."
    sudo cp crates/edgerun-vfs/edgerun-vfs.toml.example "$CONFIG_DIR/config.toml"
    echo "Edit $CONFIG_DIR/config.toml to add your directories"
else
    echo "Config already exists, skipping..."
fi

# Install systemd service
echo "Installing systemd service..."
sudo cp crates/edgerun-vfs/edgerun-vfs@.service "$SERVICE_DIR/"
sudo systemctl daemon-reload

echo
echo "=== Installation Complete ==="
echo
echo "Next steps:"
echo "1. Edit $CONFIG_DIR/config.toml to add your directories"
echo "2. Create .edgekeep files in your project directories"
echo "3. Enable the service: sudo systemctl enable edgerun-vfs@$USER.service"
echo "4. Start the service: sudo systemctl start edgerun-vfs@$USER.service"
echo "5. Check status: systemctl status edgerun-vfs@$USER.service"
echo
echo "Optional: Setup ZRAM for compression"
echo "  sudo apt install zram-tools  # Debian/Ubuntu"
echo "  sudo systemctl enable zram-tools"
echo
