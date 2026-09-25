# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Lovense WebSocket protocol support for bidirectional and streaming toy control.
- Interaction in cli to control connections and toy control (e.g. stop all toys)
- Adding GUI next to CLI to add more usability to the application
- In app config and settings to control connections next to environment variables 

---

## [0.1.1] - 2026-09-21

### Added

- **Linux Binaries:** Official pre-built release binaries for **Linux** platforms alongside Windows.
- **Text fixes** Small textual changes in documentation and build configuration. Clarification and expanding with new information for future updates.**
## [0.1.0] - 2026-09-17

### Added
- **Initial Public Release**: Pre-compiled binary release for Windows (`.exe`).
- **HTTP & HTTPS Server**:
  - Concurrent HTTP server on port `20010` and HTTPS server on port `30010`.
  - In-memory generation of self-signed TLS certificates for zero-config HTTPS compatibility.
  - Endpoints supported: `/command`, `/api/lan/v2/command`, and `/` fallback.
- **Lovense Command Set**:
  - `GetToys`: Discovers Buttplug devices and reports capabilities, statuses, and battery levels according to the Lovense standard format.
  - `GetToyName`: Lists names of available connected devices.
  - `Function`: Comprehensive actuator control (`Vibrate`, `Rotate`, `Oscillate`, `Position`, etc.) with durations and loop intervals (`loopRunningSec`, `loopPauseSec`).
  - `Position`: Normalized positioning and stroke controls.
  - `Pattern`: Parses strength rules and multi-step strength sequences.
  - `PatternV2`: Action timelines with `Setup`, `Play`, `InitPlay`, `Stop`, and `SyncTime` support.
  - `Preset`: Pre-configured vibration waveforms (`pulse`, `wave`, `fireworks`, `earthquake`).
- **Intiface / Buttplug Integration**:
  - Asynchronous WebSocket connection to Intiface Central / `intiface-engine`.
  - Automatic scanning and dynamic device discovery.
  - Smooth command translation across diverse actuator capabilities.
- **Concurrency & Responsiveness**:
  - Background asynchronous task cancellation when overriding commands are received.
  - Safe multi-threaded request routing via Tokio and Axum.
- **Configuration & Diagnostics**:
  - Environment variable overrides: `INTIFACE_WS_URL`, `LOVENSE_HTTP_PORT`, `LOVENSE_HTTPS_PORT`, and `LOVENSE_DEBUG`.
  - Detailed connection tracking and optional request/response debug logging.
- **Test Suite**:
  - Full suite of integration tests verifying command parsing, device serialization, TLS configuration, and connection tracking.
