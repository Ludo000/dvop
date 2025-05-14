# Basado Text Editor

## Overview
Basado Text Editor is a simple text editor built using GTK4 and Rust. It provides basic text editing functionality along with an embedded terminal and syntax highlighting for various programming languages. This guide provides steps to install dependencies, build, install, and run the application on Linux (Ubuntu), macOS, and Windows.

## Dependencies
The editor uses the following main Rust crates:
- GTK4 for the user interface
- VTE4 for the embedded terminal
- GtkSourceView5 for syntax highlighting
- Various other GTK-related libraries for UI components

Each platform requires specific system libraries to be installed before building the application.

## Prerequisites

### For Linux (Ubuntu 25.04):
  #### Install dev dependencies:
   Run the following commands to install the necessary dependencies:

   ```bash
   sudo apt update
   sudo apt install build-essential libgtk-4-dev libvte-2.91-dev libglib2.0-dev pkg-config libgtk-4-dev libpango1.0-dev libgtksourceview-5-dev
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
   brew install gtk4 vte3 gtksourceview5
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
   pacman -S mingw-w64-x86_64-gtk4 mingw-w64-x86_64-vte3 mingw-w64-x86_64-gtksourceview5
   ```

2. Add the MSYS2 binaries to your PATH
3. Install Rust for Windows from https://rustup.rs/

For detailed Windows setup instructions, see the [GTK-rs book](https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html).

## Building the Application
Clone the repository:
   ```bash
    git clone https://github.com/Ludo000/text-editor.git 
    cd text-editor/
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
Basado Text Editor now includes syntax highlighting for a wide range of programming languages:
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

## Installing the Application

After building, you can install the application system-wide by running:
   ```bash
    sudo cargo install --path .
   ```
You can now run basado-text-editor from any terminal.
