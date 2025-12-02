# Dvop

<p align="center">
  <img src="dvop.svg" alt="Dvop Logo" width="128" height="128">
</p>

<p align="center">
  <strong>A modern, folder-oriented code editor built with Rust and GTK4</strong>
</p>

<p align="center">
  <em>Designed for Linux, built for everywhere</em>
</p>

---

## About

**Dvop** is a lightweight, cross-platform code editor that brings the power and safety of Rust together with the native look and feel of GTK4. Built around a folder-oriented workflow, Dvop helps developers navigate and edit their projects with ease, offering a clean, responsive interface that feels at home on Linux while maintaining compatibility with macOS and Windows.

### Why Dvop?

- **Folder-First Philosophy**: Everything revolves around your project directory - file browser, terminal, and navigation all stay in sync
- **Native & Fast**: No Electron bloat - pure Rust performance with GTK4's native rendering
- **Built for GTK Development**: First-class support for GTK UI files with specialized linting and validation
- **Media-Aware**: Edit code alongside previewing markdown, SVG, images, audio, and video
- **Linux First, Everywhere Else**: Optimized for Linux but runs on macOS and Windows

## What Makes Dvop Different

### 📁 **True Folder-Oriented Workflow**
- **Smart Directory Tracking**: Automatically switches context to your active file's directory
- **Breadcrumb Navigation**: Click through folder hierarchies with visual path buttons
- **Directory-Aware Terminal**: New terminals open in your current working directory

### 🎨 **Native Media Handling**
- **Live Markdown Preview**: Side-by-side editing with real-time rendered preview
- **SVG Editor**: Edit and preview SVG files simultaneously
- **Built-in Media Players**: Audio waveform visualization and video playback without external tools
- **Image Viewer**: Browse images directly in the editor

### 🔧 **GTK UI Development Tools**
- **GTK UI File Linting**: Specialized validation for `.ui` files
- **Live UI Preview**: See GTK interface changes as you code
- **Widget Hierarchy Validation**: Catch GTK-specific errors before runtime

### ⚡ **Rust-Native Performance**
- **Instant Startup**: No electron, no overhead - just native compiled code
- **Minimal Memory Footprint**: Built with Rust's zero-cost abstractions
- **True Native Integration**: Uses your system's GTK theme and follows platform conventions

---

## Installation

### Prerequisites

Dvop requires GTK4, VTE, GtkSourceView5, and GStreamer libraries. Below are platform-specific installation instructions.

#### **Linux (Ubuntu/Debian)**

1. **Install system dependencies:**
   ```bash
   sudo apt update
   sudo apt install -y build-essential \
       libgtk-4-dev \
       libvte-2.91-dev \
       libvte-2.91-gtk4-dev \
       libglib2.0-dev \
       libpango1.0-dev \
       libgtksourceview-5-dev \
       libwebkitgtk-6.0-dev \
       libjavascriptcoregtk-6.0-dev \
       libsoup-3.0-dev \
       libgstreamer1.0-dev \
       libgstreamer-plugins-base1.0-dev \
       gstreamer1.0-plugins-good \
       gstreamer1.0-plugins-bad \
       gstreamer1.0-plugins-ugly \
       gstreamer1.0-libav \
       pkg-config
   ```

2. **Install Rust (if not already installed):**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

#### **macOS**

1. **Install dependencies via Homebrew:**
   ```bash
   brew install gtk4 vte3 gtksourceview5 \
       gstreamer gst-plugins-base gst-plugins-good \
       gst-plugins-bad gst-plugins-ugly gst-libav
   ```

2. **Install Rust (if not already installed):**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env
   ```

#### **Windows**

1. **Install MSYS2** from [https://www.msys2.org/](https://www.msys2.org/)

2. **Open MSYS2 terminal and install dependencies:**
   ```bash
   pacman -S mingw-w64-x86_64-gtk4 \
       mingw-w64-x86_64-vte3 \
       mingw-w64-x86_64-gtksourceview5 \
       mingw-w64-x86_64-gstreamer \
       mingw-w64-x86_64-gst-plugins-base \
       mingw-w64-x86_64-gst-plugins-good \
       mingw-w64-x86_64-gst-plugins-bad \
       mingw-w64-x86_64-gst-plugins-ugly \
       mingw-w64-x86_64-gst-libav
   ```

3. **Add MSYS2 binaries to your PATH**

4. **Install Rust for Windows** from [https://rustup.rs/](https://rustup.rs/)

For detailed Windows setup, see the [GTK-rs installation guide](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html).

---

## Building from Source

1. **Clone the repository:**
   ```bash
   git clone https://github.com/Ludo000/dvop.git
   cd dvop
   ```

2. **Build the application:**
   ```bash
   cargo build --release
   ```

3. **Run Dvop:**
   ```bash
   cargo run --release
   ```

**Troubleshooting:** If you encounter GTK path issues, try:
```bash
unset GTK_PATH
cargo run --release
```

---

## Installing Dvop

### **Recommended: Full Installation (Linux/macOS)**

For complete desktop integration including application icon, menu entry, and system-wide accessibility:

```bash
./install.sh
```

This will:
- Install the `dvop` binary to `~/.cargo/bin/`
- Add a desktop file for your application menu
- Install application icons (SVG and various sizes)
- Update icon and desktop caches
- Enable launching from your application launcher

After installation:
- Launch from your application menu/launcher
- Run `dvop` from any terminal
- Open files from your file manager with Dvop

### **Alternative: Binary Only Installation**

To install just the executable without desktop integration:

```bash
cargo install --path .
```

### **Uninstalling**

To completely remove Dvop:

```bash
./uninstall.sh
```

---

## Usage

### Opening a Project

**From the command line:**
```bash
# Open Dvop in the current directory
dvop

# Open Dvop in a specific directory
dvop /path/to/your/project

# Open a specific file
dvop /path/to/file.rs
```

**From the application:**
1. Click the folder icon or press `Ctrl+O` to open a file
2. Use the file explorer sidebar to navigate your project
3. Click on files in the explorer to open them

### Essential Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save file |
| `Ctrl+Shift+S` | Save as |
| `Ctrl+W` | Close tab |
| `Ctrl+Q` | Quit application |
| `Ctrl+F` | Find in file |
| `Ctrl+H` | Find and replace |
| `Ctrl+Shift+F` | Global search (search in all files) |
| `Ctrl+P` | File switcher |
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+G` | Go to line |
| `Ctrl+T` | New terminal tab |
| `Ctrl+/` | Toggle comment |
| `Alt+Up/Down` | Move line up/down |
| `Ctrl++` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size |

For a complete list of shortcuts, see [FEATURES.md](FEATURES.md).

---

## Technology Stack

Dvop is built with modern, robust technologies:

- **[Rust](https://www.rust-lang.org/)**: Systems programming language focusing on safety, speed, and concurrency
- **[GTK4](https://www.gtk.org/)**: Cross-platform toolkit for creating graphical user interfaces
- **[VTE](https://gitlab.gnome.org/GNOME/vte)**: Terminal emulator widget for the integrated terminal
- **[GtkSourceView5](https://wiki.gnome.org/Projects/GtkSourceView)**: Text editor widget with syntax highlighting
- **[GStreamer](https://gstreamer.freedesktop.org/)**: Multimedia framework for audio/video playback
- **[pulldown-cmark](https://github.com/raphlinus/pulldown-cmark)**: CommonMark parser for Markdown preview
- **[tree-sitter](https://tree-sitter.github.io/)**: Parser generator for code analysis
- **[rust-analyzer](https://rust-analyzer.github.io/)**: Language server for Rust

---

## Project Status

Dvop is under active development. Current version: **0.1.0**

- ✅ **192+ features implemented**
- ✅ **284 tests** (88 unit + 195 E2E + 1 integration)
- ✅ **15+ languages supported**
- � Additional language servers in progress
- 🚧 Plugin system planned

See [FEATURES.md](FEATURES.md) for a comprehensive feature list and [TESTING_STRATEGY.md](TESTING_STRATEGY.md) for testing documentation.

---

## Contributing

Contributions are welcome! Whether it's bug reports, feature requests, or code contributions, your help is appreciated.

**Before contributing:**
1. Check existing issues to avoid duplicates
2. Read the code to understand the architecture
3. Run tests: `./run_all_tests.sh`
4. Follow Rust best practices and GTK conventions

**Development workflow:**
```bash
# Run in debug mode
cargo run

# Run tests
cargo test
./run_e2e_tests.sh

# Format code
cargo fmt

# Run linter
cargo clippy
```

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**.

See the [LICENSE.txt](LICENSE.txt) file for full details.

---

## Legal Notice

This project was developed entirely on personal time and equipment, outside of any professional or contractual duties.  
It was not created as part of any employment relationship, nor under any instructions or using any resources, materials, or information belonging to any employer or client.

**Copyright © 2025 Ludovic Scholz**

The author retains full copyright ownership of the original source code.
