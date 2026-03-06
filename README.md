# Display Analyzer

A Linux display diagnostic tool that shows information about the entire display chain: GPU, driver, connector, protocol, and display. This helps debug display issues where any link in the chain could be the bottleneck.

Displays involve many interacting subsystems:

- **Connection**: HDMI, DisplayPort, eDP, USB-C DP Alt Mode, etc.
- **Protocol**: Each connection type has multiple versions with different feature sets and optional capabilities.
- **Display**: Resolution, refresh rate, color depth, HDR support, physical size.
- **Graphics card & driver**: Different GPUs and drivers support different protocols, features, and mediums.

When something doesn't work, it's hard to pinpoint the root cause because every component in the chain must support the feature. Display Analyzer reads all of this information from sysfs and presents it together so you can see the full picture at a glance.

## Usage

### CLI mode (default)

```
display-analyzer
```

Prints a tree of all GPUs and their connectors:

```
GPU: Intel (Intel i915) [card1] (vendor=0x8086, device=0x7dd5)
├── card1-DP-1 [disconnected]
├── card1-DP-2 [disconnected]
├── card1-DP-3 [disconnected]
├── card1-DP-4 [disconnected]
└── card1-eDP-1 [connected, enabled]
    ├── Display: BOE NE135A1M-NY1
    ├── Native: 2880x1920
    ├── Color: 8-bit, digital
    ├── Size: 29cm x 19cm (13.6")
    ├── EDID: v1.4, year 2023, week 52
    ├── Protocol: eDP
    ├── DPMS: On
    └── Modes: 2880x1920, 2880x1920
```

### GUI mode

```
display-analyzer --gui
```

Opens a graphical window (egui) showing the same information in a scrollable, collapsible layout. Connected displays are expanded by default; disconnected ones are collapsed. Click "Rescan" to re-read sysfs.

### What it reports

For each GPU:
- Driver name (i915, amdgpu, nouveau, etc.)
- PCI vendor and device IDs

For each connector:
- Connection status and power state (DPMS)
- Connector type (eDP, DP, HDMI-A, etc.)
- Available display modes

For connected displays (parsed from EDID):
- Manufacturer and model name
- Native resolution
- Color bit depth
- Physical screen size and diagonal
- EDID version and manufacturing date

For DisplayPort/eDP connectors:
- Link rate and lane count (when exposed by the driver)

## Building

### With Nix (recommended)

```
nix develop
cargo build --release
```

The flake provides Rust nightly and all system dependencies (libxkbcommon, wayland, X11, Vulkan loader) needed for the egui GUI.

### Without Nix

Install system dependencies, then build with cargo:

```
# Fedora
sudo dnf install libxkbcommon-devel wayland-devel libX11-devel vulkan-loader-devel

# Ubuntu/Debian
sudo apt install libxkbcommon-dev libwayland-dev libx11-dev

cargo build --release
```

The binary is at `target/release/display-analyzer`.

## How it works

Display Analyzer reads Linux sysfs directly (`/sys/class/drm/`) rather than using DRM ioctls. This requires no special permissions beyond read access to sysfs and works without any native library dependencies for the data gathering. Each connector directory contains status, modes, EDID blobs, and other attributes that are parsed at runtime.

## Development

This project is written in Rust and is automatically built and linted on GitHub Actions. CI runs `cargo fmt --check`, `cargo clippy -D warnings`, and both debug and release builds.

The canonical way to get dependencies is using nix flakes.
