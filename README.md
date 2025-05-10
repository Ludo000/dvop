# Basado Text Editor

## Overview
Basado Text Editor is a simple text editor built using GTK4 and Rust. It provides basic text editing functionality along with an embedded terminal. This guide provides steps to install dependencies, build, install, and run the application on Linux (Ubuntu), macOS, and Windows.

## Prerequisites

### For Linux (Ubuntu):
  #### Install dev dependencies:
   Run the following commands to install the necessary dependencies:

   ```bash
   sudo apt update
   sudo apt install build-essential libgtk-4-dev libvte-2.91-dev libglib2.0-dev pkg-config libgtk-4-dev libpango1.0-dev
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
   brew install gtk4 vte3
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
???

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
## Installing the Application

After building, you can install the application system-wide by running:
   ```bash
    sudo cargo install --path .
   ```
You can now run basado-text-editor from any terminal.
