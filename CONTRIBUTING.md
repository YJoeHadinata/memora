# Contributing to Memora

Thank you for your interest in contributing to Memora!

## Guidelines

1. **Evidence-First**: Never submit changes claiming performance improvements or memory optimizations without verifiable diagnostic measurements.
2. **Mac-Native Accuracy**: Do not parse command-line output (`vm_stat`, `top`) in production code; use native Mach, sysctl, or libproc APIs.
3. **Safety First**: Never bypass process protection safeguards or implement automated process killing.
4. **Code Quality**:
   - Ensure all unit and integration tests pass: `cargo test`
   - Ensure Clippy passes with zero warnings: `cargo clippy -- -D warnings`
   - Ensure formatting is standard: `cargo fmt -- --check`
