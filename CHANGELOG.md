# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
