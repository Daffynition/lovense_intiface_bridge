# Lovense Intiface Bridge

A high-performance, lightweight bridge written in Rust that translates Lovense Standard & LAN HTTP/HTTPS API commands to [Buttplug.io](https://buttplug.io/) / [Intiface Central](https://intiface.com/central/) protocol.

This bridge enables games, visual novels (e.g., Ren'Py projects), and applications built for the Lovense API to seamlessly control any sex toy hardware supported by the Intiface / Buttplug ecosystem.

---

## Background & Origin

During development of a Ren'Py visual novel project, there was an immediate need for an effective, low-latency way to interface the game's Lovense integration with Intiface Central to maximize user device compatibility. Searching for existing solutions revealed a proof-of-concept project created by [mistletinn](https://github.com/mistletinn). 

Inspired by that proof of concept, this project was written in Rust to provide an efficient, robust, and zero-dependency solution featuring built-in TLS generation for HTTPS, asynchronous command handling, and seamless device mapping.

---

## Release Status & Roadmap

### Version 0.1.0 (Current Release)
- **Initial Public Release:** Provides full Lovense HTTP and HTTPS local LAN API emulation.
- **Platform Availability:** Pre-compiled release binaries are currently provided for **Windows only** (`.exe`). Linux and other platforms can readily build from source using `cargo build --release`.

### Upcoming Release (Roadmap)
- **Lovense WebSocket Protocol:** Full support for the Lovense WebSocket communication interface.
- **Linux Binaries:** Official pre-built release binaries for **Linux** platforms alongside Windows.
- **Improvements:** Enhancements and optimizations to the codebase and user experience.
- **Bug Fixes:** Addressing known issues and improving stability after testing by users.

---

## Features

- **Dual HTTP & HTTPS Support:** Serves HTTP on port `20010` and HTTPS on port `30010` concurrently by default.
- **Automatic TLS/HTTPS:** Dynamically generates valid self-signed TLS certificates in-memory using `rcgen` and `rustls` without requiring external certificate files or openssl.
- **Full Lovense Command Compatibility:**
  - `GetToys`: Queries connected Buttplug devices and exposes them as Lovense toys with corresponding capability lists and battery states.
  - `GetToyName`: Returns active toy names.
  - `Function`: Handles complex vibration, rotation, oscillation, and depth controls with custom time duration and loop/pause intervals (`timeSec`, `loopRunningSec`, `loopPauseSec`).
  - `Position`: Controls linear / stroke / depth positioning actuators.
  - `Pattern`: Parses strength pattern sequences (`rule` and semicolon-delimited `strength` chains).
  - `PatternV2`: Supports timeline-based pattern actions (`Setup`, `Play`, `InitPlay`, `Stop`, `SyncTime`).
  - `Preset`: Built-in waveforms including `pulse`, `wave`, `fireworks`, and `earthquake`.
- **Actuator Feature Translation:** Intelligently maps Lovense commands (`vibrate`, `rotate`, `oscillate`, `pump`, `thrusting`, `fingering`, `suction`, `position`, `stroke`, `depth`) to matching Buttplug output capabilities.
- **Async Execution & Cancellation:** Running patterns and loops are automatically cancelled when new commands arrive, ensuring instantaneous responsiveness.
- **Low Resource Usage:** Written in Rust with Tokio and Axum for minimal CPU and memory overhead.

---

## How It Works

```
+------------------------------------+
|  Game / Application (e.g. Ren'Py)  |
+------------------------------------+
                  |
         Lovense HTTP / HTTPS
          (Port 20010 / 30010)
                  v
+------------------------------------+
|    Lovense Intiface Bridge (Rust)  |
+------------------------------------+
                  |
          Buttplug WebSocket
           (ws://127.0.0.1:12345)
                  v
+------------------------------------+
|   Intiface Central / Engine        |
+------------------------------------+
                  |
              Bluetooth
                  v
+------------------------------------+
|     Connected Haptic Devices       |
+------------------------------------+
```

---

## Getting Started

### Prerequisites

1. **Intiface Central:** Download and launch [Intiface Central](https://intiface.com/central/) (or run `intiface-engine`).
2. Start the Intiface server (default WebSocket address: `ws://127.0.0.1:12345`).
3. Connect your devices in Intiface Central.

### Running the Bridge (Windows Binary)

1. Download the latest `0.1.0` release binary for Windows (`lovense_intiface_bridge.exe`).
2. Run the executable:
   ```cmd
   lovense_intiface_bridge.exe
   ```
3. The bridge will connect to Intiface Central and begin listening for Lovense HTTP/HTTPS requests on `http://127.0.0.1:20010` and `https://127.0.0.1:30010`.

---

## Configuration

The bridge can be configured via environment variables:

| Variable             | Description                                                  | Default                 |
| :------------------- | :----------------------------------------------------------- | :---------------------- |
| `INTIFACE_WS_URL`    | WebSocket URL of Intiface Central / Buttplug server          | `ws://127.0.0.1:12345`  |
| `LOVENSE_HTTP_PORT`  | Local port for Lovense HTTP server                           | `20010`                 |
| `LOVENSE_HTTPS_PORT` | Local port for Lovense HTTPS server                          | `30010`                 |
| `LOVENSE_DEBUG`      | Enable verbose HTTP request/response logging (`1` or `true`) | `false`                 |

### Example (PowerShell / CMD):
```powershell
$env:LOVENSE_HTTP_PORT="20010"
$env:INTIFACE_WS_URL="ws://127.0.0.1:12345"
.\lovense_intiface_bridge.exe
```

---

## Building from Source

To compile the binary yourself:

1. Install the Rust toolchain from [rustup.rs](https://rustup.rs/).
2. Clone the repository:
   ```bash
   git clone https://github.com/<your-username>/lovense_intiface_bridge.git
   cd lovense_intiface_bridge
   ```
3. Build the release binary:
   ```bash
   cargo build --release
   ```
4. The executable will be available at `target/release/lovense_intiface_bridge` (or `.exe` on Windows).

---

## Testing

Run the test suite using `cargo test`:

```bash
cargo test
```

---

## Acknowledgments & Credits

- Special thanks to **[mistletinn](https://github.com/mistletinn)** and **[yhapatch](https://github.com/yhapatch)** for the original proof-of-concept that demonstrated translating Lovense commands to Intiface in and the possibility todo that in Rust.
- **[Buttplug.io / Nonpolynomial](https://buttplug.io/)** for the Buttplug Rust client and Intiface Central ecosystem.
- The adult game developers and modding community.

---

## License

This project is licensed under Apache-2.0.
