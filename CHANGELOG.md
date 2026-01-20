# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] – 2026-01-20
### Added
- Lightweight OS signal detection (non-intrusive)
- Service-based OS hints (Windows / Unix-like)
- Confidence score (0–100) for scan results
- Automatic runtime cleanup of previous scan state on startup
- Improved internal scan pipeline without breaking existing architecture

### Improved
- Service detection reliability via protocol probing + fallback
- PDF export stability with full pagination support
- Exported reports now include OS hint and confidence metadata

### Fixed
- Inconsistent scan state between consecutive runs
- Partial results appearing in PDF reports
- Internal scan result handling edge cases

### Notes
- No breaking changes from v0.1.x
- Existing commands remain unchanged
- Designed as a stable foundation for future scan extensions (v0.3.x)

---

## [0.1.1] – 2026-01-09
### Added
- Scrollable multi-column scan output
- JSON export for scan results
- PDF export with pagination
- Visual EXPORT section in EVENTS panel

### Fixed
- Broken scan output layout
- PDF export truncation issue (43-port limit)
- Unintended key-triggered export

### Notes
- Export works via commands only (export json, export pdf)
- Rust edition: 2024
