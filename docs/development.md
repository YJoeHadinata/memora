# Development Guide

## Prerequisites

- macOS (macOS 12+ recommended; tested on macOS Sequoia on Apple Silicon)
- Xcode Command Line Tools: `xcode-select --install`
- Rust stable toolchain (1.80+)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/YJoeHadinata/memora.git
cd memora

# Run unit and integration tests
cargo test

# Run code style and linter checks
cargo clippy -- -D warnings
cargo fmt -- --check

# Run development application
cargo run
```

## Packaging for macOS

To bundle as a native `Memora.app` and create a distribution DMG:

```bash
# Build release app bundle
./scripts/build-macos.sh

# Package as Memora-0.1.0.dmg
./scripts/package-dmg.sh
```
