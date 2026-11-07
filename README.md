# Memora

<p align="center">
  <img src="assets/icons/icon_256.png" alt="Memora Icon" width="128" height="128" />
  <br />
  <strong>Native macOS Memory Telemetry, Process Energy Monitor, Storage Optimizer & Menu Bar Companion</strong>
  <br />
  <em>Understand Your Mac. Keep It Fast. Keep It Healthy.</em>
</p>

<p align="center">
  <a href="https://github.com/YJoeHadinata/memora/actions/workflows/ci.yml"><img src="https://github.com/YJoeHadinata/memora/actions/workflows/ci.yml/badge.svg" alt="CI Status" /></a>
  <a href="https://github.com/YJoeHadinata/memora/releases/latest"><img src="https://img.shields.io/github/v/release/YJoeHadinata/memora?logo=github&color=blue" alt="Latest Release" /></a>
  <a href="https://github.com/YJoeHadinata/memora/releases"><img src="https://img.shields.io/github/downloads/YJoeHadinata/memora/total?color=success&logo=apple" alt="Downloads" /></a>
  <img src="https://img.shields.io/badge/platform-macOS%2012.0%2B-blue?logo=apple" alt="macOS 12+" />
  <img src="https://img.shields.io/badge/arch-Apple%20Silicon%20%7C%20Intel-green" alt="Architecture" />
  <img src="https://img.shields.io/badge/language-Rust%202021-orange?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/GUI-egui%200.37-lightblue" alt="egui" />
  <img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later" />
</p>

<p align="center">
  <img src="assets/screenshots/overview.png" alt="Memora Dashboard Overview" width="840" />
</p>

---

## Overview

**Memora** is a high-performance, lightweight system monitor and storage optimizer engineered natively for macOS. Built with Rust and `egui`, Memora queries Darwin Mach VM kernel statistics and `libproc` directly to deliver Activity Monitor-grade accuracy—with zero background bloat, zero telemetry, and zero misleading "RAM booster" gimmicks.

---

## Key Features

### 1. Memory Overview & Live Telemetry
- **Activity Monitor Reconciliation**: Computes App Memory, Wired, Compressed, Cached Files, and Free memory matching exact XNU Mach VM formulas.
- **Kernel Memory Pressure**: Real-time Darwin kernel pressure level (`Normal`, `Warning`, `Critical`) with color-coded status badges.
- **Interactive Live History**: High-resolution memory timeline plotting with zoom presets (`All`, `1m`, `3m`) and pause/resume capability.
- **Unified Memory & Storage Cards**: Overview of Apple Silicon Unified RAM and mounted drive storage volumes, cleanly separating primary user drives from internal APFS helper volumes (`/System/Volumes/VM`, `Preboot`, `Recovery`).

### 2. Deep Process Manager

<p align="center">
  <img src="assets/screenshots/processes.png" alt="Memora Process Manager" width="840" />
</p>

- **Per-Process Footprint**: Measures true physical memory consumption (`ri_phys_footprint`) via `libproc`.
- **Architecture Detection**: Real-time badges for **Apple Silicon (Native ARM64)**, **Rosetta 2 (Translated Intel)**, and **Intel (x86_64)**.
- **Safe Process Termination**: Clean Safe Quit (`SIGTERM`) and confirmed Force Quit (`SIGKILL`) with kernel-level safeguards protecting critical system daemons (`launchd`, `kernel_task`, `WindowServer`, etc.).
- **Process Inspector Modal**: Detailed drill-down for PID, pageins, disk I/O reads/writes, bundle paths, and one-click "Reveal in Finder".

### 3. Cleanup & Storage Optimizer

<p align="center">
  <img src="assets/screenshots/cleanup.png" alt="Memora Cleanup & Storage" width="840" />
</p>

- **App Uninstaller & Residuals Cleaner**: Discovers all installed apps and scrubs hidden data across `~/Library/Application Support`, `~/Library/Caches`, `~/Library/Preferences`, `~/Library/Saved Application State`, and `~/Library/Containers`. Features a two-pane inspection view, category badges, and safe one-click Trash moving with system app protection.
- **Startup & Background Items Manager**: Scans user launch agents (`~/Library/LaunchAgents`), system launch agents (`/Library/LaunchAgents`), and system daemons (`/Library/LaunchDaemons`). Detects live execution states (`Running` with PID, `Idle/Loaded`, or `Disabled`), provides 1-click Enable/Disable toggling for user agents, Finder reveal, and safe Trash removal of orphaned startup plists.
- **Large & Old Files Finder**: Uncovers massive forgotten video files, archives, and untouched disk images across `Downloads`, `Documents`, `Movies`, and `Desktop` with filters for category (Archives, Disk Images, Media, Documents), size (>50MB, >100MB, >500MB, >1GB), and age (>1 month, >6 months, >1 year), complete with "Reveal in Finder" and safe "Move to Trash".
- **macOS Trash (`~/.Trash`)**: Instant calculation of Trash volume and item count, with safe 1-click emptying via confirmation modal.
- **Leftover Installers**: Automatically detects lingering `.dmg`, `.pkg`, and `.iso` disk packages in `~/Downloads` with individual selection checkboxes.
- **APFS Local Snapshots**: Identifies and purges hidden Time Machine APFS local snapshots that inflate macOS *"System Data"*.
- **Developer Simulators**: Purges orphaned and unavailable iOS/watchOS/tvOS simulator runtimes via `xcrun simctl delete unavailable`.

### 4. Application Caches & System Maintenance
- **Application & Developer Caches**: Safely scans `~/Library/Caches`, Xcode `DerivedData`, Cargo package caches, and system application logs with safety verification badges.
- **RAM Cache Purging**: Triggers official macOS `/usr/sbin/purge` to flush inactive file buffers back to free memory without destabilizing the system.
- **Quick System Diagnostics**: 1-click DNS flush (`dscacheutil -flushcache`) and QuickLook thumbnail cache reset (`qlmanage -r cache`).

### 5. macOS Menu Bar Companion & Smart Alerts

<p align="center">
  <img src="assets/screenshots/settings.png" alt="Memora Settings & Preferences" width="840" />
</p>

- **Native Menu Bar Item**: Real-time RAM status item (`NSStatusItem`) in the top macOS menu bar with a companion dropdown menu for live memory stats, quick RAM purge, Trash emptying, and 1-click dashboard access.
- **Smart High RAM Alerts**: Native Notification Center banners alerting when any single process spikes above configurable limits (2 GB, 3 GB, 4 GB, 6 GB, 8 GB).
- **Kernel Pressure Alerts**: Automatic alerts when macOS memory pressure elevates to Warning or Critical.
- **Energy Hogs & CPU Impact**: Immediate identification of battery drainers with dedicated filter and color-coded CPU usage indicators.

### 6. Native Apple Silicon Design
- **Responsive HIG Layout**: Adapts gracefully to window resizing, full-screen, or compact desktop arrangements.
- **Automatic Dark/Light Mode**: Matches macOS system appearance instantly with high-contrast, accessible typography.
- **Ultra-Lean Footprint**: Less than 1% CPU usage at 2-second sampling interval; distribution DMG installer is only **~6.8 MB**.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| <kbd>⌘</kbd> + <kbd>1</kbd> | Switch to **Overview** Tab |
| <kbd>⌘</kbd> + <kbd>2</kbd> | Switch to **Processes** Tab |
| <kbd>⌘</kbd> + <kbd>3</kbd> | Switch to **Cleanup & Storage** Tab |
| <kbd>⌘</kbd> + <kbd>4</kbd> | Switch to **Settings** Tab |
| <kbd>⌘</kbd> + <kbd>F</kbd> | Focus Process Search Filter |
| <kbd>⌘</kbd> + <kbd>E</kbd> | Export Diagnostic JSON Snapshot to `~/Downloads` |
| <kbd>⌘</kbd> + <kbd>T</kbd> | Toggle Light / Dark Theme |
| <kbd>Esc</kbd> | Close Modal Dialog or Inspector |

---

## Installation

### Option 1: Install via Homebrew (Recommended)

Install directly via the official Homebrew Tap:

```bash
brew install --cask YJoeHadinata/tap/memora
```

To update when a new version is released:

```bash
brew upgrade --cask memora
```

### Option 2: Direct Download (.dmg)

1. Download the latest `Memora-0.2.0.dmg` from the [Releases](https://github.com/YJoeHadinata/memora/releases) page.
2. Open the `.dmg` file and drag **Memora** into your `/Applications` folder.
3. *First Launch Note*: Since Memora is an open-source, non-notarized build, open it by **Right-Clicking (Control-Click)** on `Memora.app` in `/Applications` and selecting **Open**, or run:
   ```bash
   xattr -cr /Applications/Memora.app
   ```

### Option 3: Build From Source

```bash
# Clone the repository
git clone https://github.com/YJoeHadinata/memora.git
cd memora

# Run quality checks
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test

# Build and run directly
cargo run --release
```

---

## Packaging

To package a standalone `.app` bundle and compressed `.dmg` disk image:

```bash
# 1. Build .app bundle in dist/
./scripts/build-macos.sh

# 2. Package into compressed .dmg with SHA-256 checksum
./scripts/package-dmg.sh
```

Distribution output:
- `dist/Memora.app`
- `dist/Memora-0.2.0.dmg`
- `dist/Memora-0.2.0.dmg.sha256`

---

## Architecture Overview

Memora follows a strict unidirectional architecture:

```text
macOS Mach VM / sysctl / libproc / tmutil
                    │
                    ▼
           System Collectors
                    │
                    ▼
            Domain Services
                    │
                    ▼ (mpsc async channel)
           egui Application Loop (60 FPS)
                    │
                    ▼
     Overview  │  Processes  │  Cleanup  │  Settings
```

- **Thread Isolation**: All heavy metrics gathering, cache traversal, and shell invocations execute on background worker threads, keeping the immediate-mode GUI responsive at 60 FPS without stutter.
- **Safe Memory Bounds**: The timeline buffer uses bounded ring buffers (`HistoryBuffer`) to prevent memory accumulation during long multi-day monitoring sessions.

---

## Technical Documentation

- [macOS Memory Metrics Formula](docs/macos-metrics.md)
- [Architecture Details](docs/architecture.md)
- [Development Guide](docs/development.md)

---

## License

This project is licensed under the [GNU General Public License v3.0 or later (GPL-3.0-or-later)](LICENSE).

- **Free & Open Source:** You are free to use, run, study, and modify this software for personal or commercial use.
- **Copyleft Protection:** Any derivative works or modifications distributed by third parties must also be licensed under the GPLv3 with full source code made publicly available. Proprietary re-licensing, closed-source paywalling, or selling this software as a closed commercial product is strictly prohibited.
