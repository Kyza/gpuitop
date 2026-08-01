# gpuitop

A GPU-accelerated task manager for Linux built with [GPUI](https://www.gpui.rs/), the Rust UI framework behind the [Zed](https://zed.dev) editor. Fuzzy search, window picking, per-process GPU VRAM tracking (NVIDIA + AMD), and a right-click context menu for signals.

## Features

### Find Processes

Fuzzy search the process table, or click **Pick** and switch to any window to jump to its process. Toggle filters to narrow the list: GUI apps, kernel threads, systemd, Electron apps, your user, process state (R/S/D/Z/T/I/X), parents, VRAM-consuming processes. Filters combine with AND/OR logic.

The window picker uses the `zwlr_foreign_toplevel_manager_v1` Wayland protocol. It does not work under X11; the Pick button will time out in an X11 session.

### Manage Processes

Right-click any row for a context menu:

- **End Process** (SIGTERM)
- **Force Kill** (SIGKILL)
- **Pause** (SIGSTOP) or **Resume** (SIGCONT)
- **Copy PID** to clipboard
- Send arbitrary signals: SIGHUP, SIGINT, SIGQUIT, SIGUSR1, SIGUSR2

Double-click a process to filter by it. Click column headers to sort by Name, PID, User, State, CPU%, Memory%, VRAM, Disk Read, or Disk Write.

### GPU VRAM Tracking

NVIDIA GPUs are queried through NVML. AMD GPUs use `rocm-smi`. Both backends aggregate across all GPUs and show per-process VRAM usage.

### Electron Detection

Discord, VS Code, and other Electron apps show their real name instead of "electron". The detector walks the process tree to find the root app name.

### System Resources

The Performance tab shows CPU (overall bar + per-core gauges), memory (total, used, available, cached, swap), disk I/O per device, and network throughput per interface.

## Prerequisites

- Rust toolchain: [rustup](https://rustup.rs)
- Linux with X11 or Wayland
- GPUI build dependencies:

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

- NVIDIA VRAM: NVIDIA driver (NVML bundled with the driver)
- AMD VRAM: `rocm-smi` from the ROCm stack

## Install

```bash
cargo install --git https://github.com/Kyza/gpuitop.git
```

## Platform Support

Linux only. The codebase is structured with platform abstraction (`src/platform/`) so the process collector, GPU queries, and window picker can be swapped per OS. Help with Windows support is welcome.

## Config

Stored at `~/.config/gpuitop/config.ron` (respects `XDG_CONFIG_HOME`). Editable in-app under Settings, or directly in the RON file.
