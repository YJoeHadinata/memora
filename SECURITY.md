# Security Policy

## Supported Versions

We actively maintain and provide security updates for the latest release series of Memora.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

---

## Reporting a Vulnerability

The Memora project takes the security, stability, and privacy of your system seriously. If you discover a security vulnerability or privilege escalation issue in Memora, please report it responsibly through private disclosure.

### How to Report

- **Preferred Method**: Open a [Private Vulnerability Report](https://github.com/YJoeHadinata/memora/security/advisories/new) directly through GitHub Security Advisories.
- **Alternative Method**: Email the maintainer directly at **`yoshuajoeh@gmail.com`** with the subject line `[SECURITY] Memora Vulnerability Report`.

Please **do not** report security vulnerabilities via public GitHub issues, discussions, or social media.

### What to Include

To help us investigate and resolve the issue quickly, please include:
1. **Description**: A clear summary of the vulnerability and its potential impact.
2. **Environment**: Your macOS version (e.g., macOS 14 Sonoma, macOS 15 Sequoia) and hardware architecture (Apple Silicon ARM64 or Intel x86_64).
3. **Reproduction Steps**: Step-by-step instructions or a minimal Proof of Concept (PoC) to reproduce the behavior.
4. **Proposed Fix**: Any suggested mitigations or patches (optional).

### Response Timeline

- **Initial Acknowledgment**: Within **48 hours** of receiving your report.
- **Triage & Status Update**: Within **5 business days**, confirming whether the issue can be reproduced and discussing remediation steps.
- **Fix & Public Advisory**: Once a fix is verified, a patch release and security advisory will be published simultaneously, crediting the reporter (unless anonymity is requested).

---

## Security Architecture & Privacy Guarantees

Memora is engineered with strict privacy and security principles:

1. **100% Offline & Zero Telemetry**:
   - Memora does **not** make incoming or outgoing network calls.
   - It contains zero telemetry, analytics, third-party tracking, or crash-reporting SDKs.
   - Any external network request originating from Memora would indicate a severe bug or compromise.

2. **Least Privilege Principle**:
   - Memora runs strictly under standard user space permissions.
   - It never requests `root` / `sudo` privileges and does not install background daemon helpers or kernel extensions (`kext`).

3. **System Process Protection Safeguards**:
   - The process manager enforces hardcoded, non-overridable safeguards preventing the termination (`SIGTERM` / `SIGKILL`) of critical macOS system processes (e.g., `PID <= 1`, `kernel_task`, `launchd`, `WindowServer`, `loginwindow`, and `securityd`).

4. **Transparent Binary Distribution**:
   - All macOS `.app` bundles and `.dmg` disk images distributed via GitHub Releases include published SHA-256 checksums generated during automated CI builds for tamper verification.
