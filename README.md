# WISE1738

WISE1738 is a minimal terminal-based TCP port scanner written in Rust with a custom TUI.

Created by Diamond.

---

## Purpose

This project was built to demonstrate:
- Real TCP port scanning logic
- Correct handling of open / closed / filtered ports
- Clean Rust architecture
- Custom terminal UI engineering

The focus is on accuracy and transparency, not feature quantity.

---

## What It Does NOT Do

WISE1738 intentionally avoids:
- Exploitation or brute force
- Fake or assumed scan results
- External scanners (nmap, masscan)
- OpenSSL or heavy TLS libraries
- Raw packet / SYN scanning

These choices keep the project simple, auditable, and portfolio-friendly.

---

## Features

- Real TCP connect scanning
- Parallel scanning
- IP and domain support
- Flexible port input:
  - 80
  - 22,80,443
  - 1-1024
- Basic service detection (HTTP, HTTPS, SSH, MySQL, RDP)
- Minimal TLS ClientHello probing
- Scrollable terminal UI
- Timestamped event log

---


## Build & Run

### Requirements
- Rust (stable)
- Linux (Debian/Ubuntu recommended)

### Build (recommended)
`bash

cargo build --release
./target/release/wise1738

Run with Cargo (development)

cargo run

Run with Cargo (release mode)

cargo run --release




Usage
Inside the terminal:


scan <ip|domain>
scan <ip|domain> 80
scan <ip|domain> 22,80,443
scan <ip|domain> 1-1024
exit


Example:

scan example.com 80-443
