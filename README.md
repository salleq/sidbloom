# SIDbloom

A modern, free and open-source 3-channel SID music tracker for Windows and macOS.
Compose C64 music with a clearer interface and export real, C64-compatible `.sid` files.

**Status:** early development. Done (`core/`, all tested):
- song model, tick-based playback engine, project files (`.sidb`)
- 6502 player + `.sid` (PSID v2) / `.prg` export, verified register-for-register against the engine in a 6502 emulator and played back by the independent `sidplayfp`
- offline audio rendering with reSID (WAV)

Planned: Tauri/React UI, realtime audio output, SID-Wizard `.swm` import.

## Try it
```
cd core
cargo run --release --bin sidbloom -- demo demo.sidb
cargo run --release --bin sidbloom -- render demo.sidb demo.wav 8
cargo run --release --bin sidbloom -- export-sid demo.sidb demo.sid   # plays on a real C64 / any SID player
```

License: GPL-3.0-or-later (see `LICENSE`, `NOTICE.md`).
