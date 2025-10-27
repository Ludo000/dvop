#!/bin/bash
# Uninstall script for Dvop text editor

echo "Uninstalling Dvop..."

# Remove the binary
cargo uninstall dvop 2>/dev/null || true

# Remove desktop file
DESKTOP_FILE="$HOME/.local/share/applications/com.example.Dvop.desktop"
if [ -f "$DESKTOP_FILE" ]; then
    rm "$DESKTOP_FILE"
    echo "Removed desktop file"
fi

# Remove icon files
ICON_BASE_DIR="$HOME/.local/share/icons/hicolor"
SVG_DIR="$ICON_BASE_DIR/scalable/apps"
if [ -f "$SVG_DIR/dvop.svg" ]; then
    rm "$SVG_DIR/dvop.svg"
    echo "Removed SVG icon"
fi

PIXMAPS_DIR="$HOME/.local/share/pixmaps"
if [ -f "$PIXMAPS_DIR/dvop.svg" ]; then
    rm "$PIXMAPS_DIR/dvop.svg"
    echo "Removed pixmaps icon"
fi

# Update icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$ICON_BASE_DIR" 2>/dev/null || true
fi

# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

echo ""
echo "✓ Uninstallation complete!"
echo ""
echo "Note: Configuration files in ~/.config/dvop/ were not removed."
echo "To remove them: rm -rf ~/.config/dvop/"
