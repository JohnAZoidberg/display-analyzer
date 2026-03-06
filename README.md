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
$ display-analyzer
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
    │   ├── AUX Channel: AUX A/DDI A/PHY A
    │   └── DPCD: not readable (requires root for /dev/drm_dp_aux*)
    ├── DPMS: On
    └── Modes: 2880x1920, 2880x1920

$ sudo display-analyzer
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
    │   ├── AUX Channel: AUX A/DDI A/PHY A
    │   ├── DP Version: 1.4
    │   ├── Max Link Rate: 5.4 Gbps/lane (HBR2, 0x14)
    │   ├── Max Lanes: 4
    │   ├── Max Bandwidth: 21.6 Gbps total (17.3 Gbps effective)
    │   ├── Capabilities: enhanced framing, TPS3, 0.5% downspread
    │   ├── Active Link: 5.4 Gbps/lane (HBR2, 0x14) x 4 lanes
    │   ├── Active Bandwidth: 21.6 Gbps total (17.3 Gbps effective)
    │   ├── Sink Count: 1
    │   ├── Lane 0: CR=ok EQ=FAIL Lock=FAIL
    │   ├── Lane 1: CR=ok EQ=FAIL Lock=FAIL
    │   ├── Lane 2: CR=ok EQ=FAIL Lock=FAIL
    │   ├── Lane 3: CR=ok EQ=FAIL Lock=FAIL
    │   ├── Interlane Align: FAIL
    │   ├── PSR: PSR2 (Y-coordinate)
    │   │   ├── State: PSR2 enabled
    │   │   ├── Sink Status: active (RFB)
    │   │   ├── Setup Time: 55 us
    │   │   ├── SU Granularity: 0x4 pixels
    │   │   ├── Features: Y-coordinate required, SU granularity required
    │   │   └── Errors: none
    │   └── PSR Driver Status:
    │       Sink support: PSR = yes [0x03], Panel Replay = no, Panel Replay Selective Update = no
    │       PSR mode: PSR2 enabled
    │       Source PSR/PanelReplay ctl: enabled [0x80004a26]
    │       Source PSR/PanelReplay status: DEEP_SLEEP [0x80000100]
    │       Busy frontbuffer bits: 0x00000000
    │       Performance counter: 0
    │       PSR2 selective fetch: enabled
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
nix build
./result/bin/display-analyzer
```

Or run directly:

```
nix run
```

For development, enter the dev shell:

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

## DisplayPort link training

DisplayPort connections go through a link training process where the transmitter (GPU) and receiver (display) negotiate signal parameters. The tool shows per-lane training status with three phases:

1. **CR (Clock Recovery)** — The receiver locks onto the transmitter's clock signal. This is the first step; without it, nothing else works. Failures here usually indicate a bad cable or connector.

2. **EQ (Channel Equalization)** — The signal quality is tuned so the receiver can reliably distinguish 0s from 1s at the target data rate. The transmitter adjusts voltage swing and pre-emphasis until the signal is clean. Failures often mean the cable is too long or low quality for the link rate.

3. **Lock (Symbol Lock)** — The receiver locks onto symbol boundaries in the data stream, confirming the link can carry actual data.

These phases happen in order — CR must pass before EQ is attempted, and EQ before symbol lock. **Interlane Align** is a final check that all lanes are synchronized so multi-lane data can be reassembled correctly.

Transient failures (CR=ok, EQ=FAIL, Lock=FAIL) are normal during PSR (Panel Self Refresh) transitions — the link wakes up, clock recovers quickly, but equalization hasn't completed yet. These resolve within milliseconds.

## How it works

Display Analyzer reads Linux sysfs directly (`/sys/class/drm/`) rather than using DRM ioctls. This requires no special permissions beyond read access to sysfs and works without any native library dependencies for the data gathering. Each connector directory contains status, modes, EDID blobs, and other attributes that are parsed at runtime.

## Development

This project is written in Rust and is automatically built and linted on GitHub Actions. CI runs `cargo fmt --check`, `cargo clippy -D warnings`, and both debug and release builds.

The canonical way to get dependencies is using nix flakes.
