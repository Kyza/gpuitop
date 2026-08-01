# gpuitop

A Linux-first desktop system monitor and process manager with GPU VRAM tracking, built with the [GPUI](https://www.gpui.rs/) framework.

![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Process table** with sortable columns: Name, PID, User, State, CPU%, Memory%, VRAM, Disk Read, Disk Write
- **GPU VRAM tracking** for NVIDIA (NVML) and AMD (rocm-smi) — per-process memory usage across all GPUs
- **Fuzzy search and filtering** — toggle filters for GUI apps, kernel threads, systemd, Electron apps, process state, and more
- **Electron app detection** — shows actual app names (Discord, VS Code) instead of just "electron"
- **Performance tab** — CPU utilization, per-core gauges, memory breakdown, disk I/O, and network throughput
- **Hierarchical process view** — cumulative or self-only resource view with parent/child expansion
- **Dark, light, and system themes** — toggle from the title bar
- **Persistent configuration** saved as RON at `~/.config/gpuitop/config.ron`

## Screenshot

<!-- TODO: add screenshot -->

## Prerequisites

- **Rust** toolchain (install via [rustup](https://rustup.rs))
- **Linux** with X11 or Wayland
- Build dependencies for GPUI:

```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config cmake \
  libx11-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev libfontconfig-dev libvulkan-dev

# Fedora
sudo dnf install gcc-c++ cmake pkg-config \
  libX11-devel libxkbcommon-devel libxkbcommon-x11-devel \
  wayland-devel fontconfig-devel vulkan-devel

# Arch
sudo pacman -S base-devel cmake pkgconf \
  libx11 libxkbcommon libxkbcommon-x11 \
  wayland fontconfig vulkan-headers
```

- For **NVIDIA VRAM tracking**: NVIDIA driver with NVML (installed with the driver)
- For **AMD VRAM tracking**: `rocm-smi` from the ROCm stack

## Install

```bash
cargo install --git https://github.com/Kyza/gpuitop.git
```

Then run:

```bash
gpuitop
```

## Usage

| Action | Binding |
|--------|---------|
| Switch tabs | Click Processes / Performance / Settings |
| Sort columns | Click column headers |
| Search processes | Type in the search bar |
| Toggle filters | Click filter buttons below the search bar |
| Filter by process | Double-click a process row |
| Pin a process | Right-click → Pin |
| Change theme | Click the sun/moon icon in the title bar |
| Edit config | Settings → About → Open Config File |

## Config

Configuration is stored at `$XDG_CONFIG_HOME/gpuitop/config.ron` (defaults to `~/.config/gpuitop/config.ron`). All settings can also be changed through the in-app Settings tab.
