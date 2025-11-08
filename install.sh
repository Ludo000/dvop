#!/bin/bash
# Install script for Dvop text editor
#test

set -e

echo "Installing Dvop..."

# Install the binary
cargo install --path .

# Install desktop file
DESKTOP_FILE="com.example.Dvop.desktop"
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"
cp "$DESKTOP_FILE" "$DESKTOP_DIR/"
echo "Installed desktop file to $DESKTOP_DIR/"

# Install icon files
ICON_SIZES="16 22 24 32 48 64 128 256"
ICON_BASE_DIR="$HOME/.local/share/icons/hicolor"

# For SVG (scalable)
SVG_DIR="$ICON_BASE_DIR/scalable/apps"
mkdir -p "$SVG_DIR"
cp dvop.svg "$SVG_DIR/"
echo "Installed SVG icon to $SVG_DIR/"

# Also install to pixmaps as fallback
PIXMAPS_DIR="$HOME/.local/share/pixmaps"
mkdir -p "$PIXMAPS_DIR"
cp dvop.svg "$PIXMAPS_DIR/"
echo "Installed icon to $PIXMAPS_DIR/"

# Update icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$ICON_BASE_DIR" 2>/dev/null || true
    echo "Updated icon cache"
fi

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    echo "Updated desktop database"
fi

echo ""
echo "✓ Installation complete!"
echo "You can now launch Dvop from your application menu or by running 'dvop'"
echo ""
echo "To uninstall, run: ./uninstall.sh"
