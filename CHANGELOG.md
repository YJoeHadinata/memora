# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-07

### Added
- **App Uninstaller & Residuals Cleaner**:
  - Full application scanner across `/Applications`, `~/Applications`, and `/System/Applications`.
  - Automatic `CFBundleIdentifier`, `CFBundleDisplayName`, and version resolution from `.app` bundles via fast in-memory XML plist parser with `plutil` fallback.
  - Deep residual scanner inspecting `~/Library/Application Support`, `~/Library/Caches`, `~/Library/Preferences`, `~/Library/Saved Application State`, `~/Library/Containers`, `~/Library/HTTPStorages`, `~/Library/WebKit`, and `~/Library/Logs`.
  - Interactive two-pane interface: search and select applications sorted by total footprint on the left, inspect detailed file-by-file breakdown with category pills on the right.
  - Safe one-click uninstalling moving bundles and leftovers directly to macOS Trash via AppleScript Finder integration with confirmation dialog.
  - System application safeguards preventing accidental deletion of native macOS utilities and protected system apps.
- **Startup & Background Services Manager (LaunchAgents & LaunchDaemons)**:
  - Comprehensive scan across user launch agents (`~/Library/LaunchAgents`), system-wide launch agents (`/Library/LaunchAgents`), and root system daemons (`/Library/LaunchDaemons`).
  - Real-time `launchctl` session synchronization to detect active daemon execution state (Running with PID, Loaded/Idle, Disabled, or Not Loaded).
  - Reverse-DNS service label resolution and friendly display name mapping (Google Keystone, Chrome Updater, Steam Clean, PostgreSQL, Cloudflare WARP, Spotify, etc.).
  - Interactive status controls: one-click Disable/Enable toggle for user launch agents via safe `.plist.disabled` renaming and `launchctl` loading.
  - Safe removal of orphaned or unwanted background service plists to macOS Trash with confirmation dialog.
  - Quick "Finder" reveal button to inspect service plists in native macOS Finder.
  - Instant filtering by status (All, Running Only, User Agents, System Daemons) and keyword search.

### Changed
- **Modular Architecture & Code Smell Elimination**:
  - Refactored and decomposed all oversized modules (`app`, `ui/caches`, `ui/dashboard`, `ui/processes`, `ui/helpers`, `system/macos/process`, `system/macos/menu_bar`) so every Rust source file in `src/` is strictly `<= 200` lines.
  - Centralized duplicated filesystem sizing, Finder reveal, macOS Trash moving, and subprocess helpers into `src/utils/fs_ops.rs`.
  - Added fast in-memory XML `<key>` plist parser in `src/utils/plist.rs` (eliminating hundreds of `plutil` subprocess spawns during app and startup scanning) and per-PID binary architecture caching (`arch_cache`) in `MacosProcessCollector`.
  - Dynamic version badge resolution via `env!("CARGO_PKG_VERSION")` in Settings and Keyboard Shortcuts modals.
- Updated project licensing to **GNU General Public License v3.0 or later (GPL-3.0-or-later)**.
- Fixed layout overlap in Storage Optimizer cards by implementing proportional container allocation and automatic text wrapping.
- Improved row height and vertical padding in the App Uninstaller list, eliminating selection box clipping.

## [0.1.0] - 2026-11-07

### Added
- **Core Memory Telemetry**:
  - Direct Mach VM kernel query via `host_statistics64` with `HOST_VM_INFO64`.
  - Exact reconciliation with macOS Activity Monitor formulas (App Memory, Wired, Compressed, Cached, Free).
  - Real-time kernel memory pressure monitoring via `kern.memorystatus_vm_pressure_level`.
  - Swap memory tracking via `sysctl` `vm.swapusage`.
  - Interactive live history plotting with time range presets (`All`, `1m`, `3m`).
- **Process Management & Energy Monitoring**:
  - Per-process physical footprint inspection (`ri_phys_footprint`) via `libproc`.
  - Architecture detection: **Apple Silicon (Native ARM64)**, **Rosetta 2 (Translated Intel)**, and **Intel (x86_64)**.
  - Energy & Battery impact calculation combining CPU utilization and disk I/O.
  - Dedicated "Energy Hogs (>10% CPU)" filter with color-coded CPU indicators.
  - Safe Quit (`SIGTERM`) and confirmed Force Quit (`SIGKILL`) with safeguards protecting critical macOS system processes (`PID <= 1`, `launchd`, `kernel_task`, `WindowServer`, etc.).
  - Process inspector modal with pageins, disk I/O, and Finder reveal.
- **Cleanup & Storage Optimizer**:
  - Large & Old Files Finder: recursive scanning of user folders (`Downloads`, `Documents`, `Movies`, `Desktop`) with Category, Size, and Age filtering, Finder reveal, and safe Trash moving.
  - macOS Trash scanner and safe 1-click cleaner for `~/.Trash` (with Finder AppleScript fallback for TCC permissions).
  - Lingering installer packages (`.dmg`, `.pkg`, `.iso`) scanner across user directories and external volumes.
  - APFS local Time Machine snapshots inspector and purger (`tmutil`).
  - Developer runtimes pruner for unavailable Xcode iOS simulators (`xcrun simctl delete unavailable`).
- **Menu Bar Companion & Smart Notifications**:
  - Native Cocoa `NSStatusItem` in the macOS top menu bar displaying live memory with companion action menu.
  - Asynchronous native Notification Center alerts for high RAM process spikes and kernel memory pressure warnings with configurable thresholds.
- **Caches & System Diagnostics**:
  - Comprehensive scan of `~/Library/Caches`, Xcode `DerivedData`, Cargo package cache, and application logs.
  - macOS system RAM buffer cache purging via `/usr/sbin/purge`.
  - DNS cache flushing (`dscacheutil`) and QuickLook thumbnail cache reset (`qlmanage`).
- **Apple Human Interface Design**:
  - Responsive window layout supporting standard, compact, and full-screen modes.
  - Automatic Dark/Light mode theme synchronization with macOS.
  - Unified Memora squircle icon displayed across Dock, top bar, and About settings.
  - Keyboard navigation shortcuts (`⌘1`–`⌘4`, `⌘F`, `⌘E`, `⌘T`, `⌘Q`, `Esc`).
- **Distribution & Quality**:
  - Standalone `.app` bundle script (`scripts/build-macos.sh`) with ad-hoc codesigning for Apple Silicon Gatekeeper compliance.
  - Compressed `.dmg` packager with SHA-256 verification (`scripts/package-dmg.sh`).
  - GitHub Actions CI and automated release packaging workflows.
