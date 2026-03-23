# Tailscale Nodes UI (Vibe Edition)

A modern, interactive graphical interface for managing your Tailscale network, built with Rust using the **Clay** layout engine and **Raylib** for high-performance rendering.

## Features

- **Real-time Node Monitoring**: Displays all nodes in your Tailnet with automatic periodic refreshes.
- **Advanced Sorting**: Automatically sorts by:
  1.  **Online Status** (Online nodes first)
  2.  **Last Seen** (Most recently active nodes first)
  3.  **Host Name** (Alphabetical)
- **Live Search**: Filter nodes instantly by hostname or IP address as you type.
- **Exit Node Management**: Dedicated buttons to toggle "Use Exit Node" or "Stop Exit Node" for supporting peers.
- **One-Click Copy**: Click any node row to instantly copy its Tailscale IP to your clipboard.
- **Embedded Aesthetics**: Completely self-contained binary with **JetBrainsMono Nerd Font** embedded for crisp typography and rich OS/Status icons.
- **Interactive Scaling**: Support for `Ctrl + "+"` and `Ctrl + "-"` to dynamically scale the entire UI.
- **Touchpad & Scroll Support**: Smooth vertical scrolling for large node lists.
- **Container Ready**: Designed to run seamlessly inside Distrobox/Toolbox, using `distrobox-host-exec` to communicate with the host's Tailscale daemon.

## Tech Stack

- **Rust**: High-performance, memory-safe systems programming.
- **Clay**: A high-performance, flexbox-like layout engine.
- **Raylib**: Simple and easy-to-use library to enjoy videogames programming.
- **Tokio**: Asynchronous runtime for periodic status updates and non-blocking Tailscale commands.

## Getting Started

### Prerequisites

Ensure you have the necessary Raylib build dependencies installed (especially if running on Linux/Distrobox):
`cmake`, `libclang-dev`, `libasound2-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libgl1-mesa-dev`, `libglu1-mesa-dev`, `libxcursor-dev`, `libxinerama-dev`.

### Running

```bash
cargo run
```

## Controls

- **Type**: Real-time search/filtering.
- **Backspace**: Delete search characters.
- **Escape**: Clear search bar.
- **Click Node Row**: Copy IP to clipboard.
- **Click Green/Red Button**: Toggle Exit Node routing.
- **Scroll/Drag**: Navigate long lists.
- **Ctrl + "+" / "-"**: Zoom UI in/out.
- **Ctrl + C**: Quit (or close the window).
