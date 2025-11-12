# Dvop

<p align="center">
  <img src="dvop.svg" alt="Dvop Logo" width="128" height="128">
</p>

## Overview
Dvop is an IDE built using GTK4 and Rust. It provides basic text editing functionality along with an embedded terminal and syntax highlighting for various programming languages. This guide provides steps to install dependencies, build, install, and run the application on Linux (Ubuntu), macOS, and Windows.

## Dependencies
The editor uses the following main Rust crates:
- GTK4 for the user interface
- VTE4 for the embedded terminal
- GtkSourceView5 for syntax highlighting
- Various other GTK-related libraries for UI components

Each platform requires specific system libraries to be installed before building the application.

## Prerequisites

### For Linux (Ubuntu 25.10):
  #### Install dev dependencies:
   Run the following commands to install the necessary dependencies:

   ```bash
   sudo apt update
   sudo apt install build-essential libgtk-4-dev libvte-2.91-dev libvte-2.91-gtk4-dev libglib2.0-dev pkg-config libgtk-4-dev libpango1.0-dev libgtksourceview-5-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav
  ```

#### Install Rust:
If you haven't installed Rust, you can do so using the following command:

   ```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
After installation, run the following command:
   ```bash
    source $HOME/.cargo/env
   ```
### For macOS:
  #### Install package dependencies:
Install Dependencies using Homebrew:
Ensure you have Homebrew installed, then run the following commands:
    
   ```bash
   brew install gtk4 vte3 gtksourceview5 gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav
   ```

#### Install Rust:
If you haven't installed Rust, you can do so using the following command:

   ```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
After installation, run the following command:
   ```bash
    source $HOME/.cargo/env
   ```

### For Windows:
Installing GTK4 and related libraries on Windows is more complex:

1. Use MSYS2 (https://www.msys2.org/) to install the required packages:
   ```bash
   pacman -S mingw-w64-x86_64-gtk4 mingw-w64-x86_64-vte3 mingw-w64-x86_64-gtksourceview5 mingw-w64-x86_64-gstreamer mingw-w64-x86_64-gst-plugins-base mingw-w64-x86_64-gst-plugins-good mingw-w64-x86_64-gst-plugins-bad mingw-w64-x86_64-gst-plugins-ugly mingw-w64-x86_64-gst-libav
   ```

2. Add the MSYS2 binaries to your PATH
3. Install Rust for Windows from https://rustup.rs/

For detailed Windows setup instructions, see the [GTK-rs book](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html).

## Building the Application
Clone the repository:
   ```bash
    git clone https://github.com/Ludo000/dvop.git 
    cd dvop/
   ```
Build the application:
   ```bash
    cargo build --release
   ```
To run the application:
   ```bash
    cargo run
   ```
If you have a GTK path related issue, try :
   ```bash
unset GTK_PATH
   ```
And then try again the ```cargo run``` command

## Features

### Syntax Highlighting
Dvop now includes syntax highlighting for a wide range of programming languages:
- Rust (.rs)
- Python (.py)
- JavaScript (.js) 
- TypeScript (.ts)
- C/C++ (.c, .cpp, .h, .hpp)
- HTML (.html)
- CSS (.css)
- Java (.java)
- Shell scripts (.sh)
- Ruby (.rb)
- PHP (.php)
- XML (.xml)
- JSON (.json)
- Markdown (.md)
- YAML (.yml, .yaml)
- TOML (.toml)
- And many more!

The editor automatically detects the file type based on its extension and applies the appropriate syntax highlighting.

### Dark Mode Support
The editor supports dark mode for comfortable coding in low-light environments:
- Automatically detects your system's theme preference
- In light mode, uses a clean, light syntax highlighting theme
- In dark mode, switches to a dark syntax highlighting theme that's easier on the eyes
- Theme changes are applied instantly when you toggle the system theme
- Includes a dedicated dark mode toggle button in the header bar for quick switching
- Intelligently selects the best available dark/light scheme for your system

### Other Features
- Multi-tab editing
- Embedded terminal
- File browser
- Basic text editing capabilities
- Audio file playback support (MP3, WAV, FLAC, OGG, M4A, AAC, OPUS, WMA)
  - Built-in audio player with play/pause/stop controls
  - Progress bar with seek functionality
  - Displays audio file information and duration
  - Powered by GStreamer for reliable audio playback

## Installing the Application

### Recommended Installation (with desktop integration)
For proper desktop integration including application icon and menu entry:
   ```bash
    ./install.sh
   ```
This will:
- Install the `dvop` binary to `~/.cargo/bin/`
- Install the desktop file for application menu integration
- Install the application icon
- Update icon and desktop caches

### Manual Installation
Alternatively, you can install just the binary:
   ```bash
    cargo install --path .
   ```

### Uninstalling
To uninstall the application:
   ```bash
    ./uninstall.sh
   ```

After installation, you can:
- Run `dvop` from any terminal
- Launch Dvop from your application menu
- Pin it to your dock/taskbar

## 📜 Legal Notice

This project was developed entirely on personal time and equipment, outside of any professional or contractual duties.  
It was not created as part of any employment relationship, nor under any instructions or using any resources, materials, or information belonging to any employer or client.

The author retains full copyright ownership of the original source code.

This project is distributed under the terms of the **GNU General Public License version 3 (GPLv3)**, as published by the Free Software Foundation.  
See the `LICENSE` file for full license details.

©Ludovic Scholz - 2025
