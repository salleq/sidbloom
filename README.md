# SIDbloom

A modern, free and open-source 3-channel SID music tracker for Windows and macOS.
Compose C64 music with a clearer interface and export real, C64-compatible `.sid` files.

**Status:** early but working. You can write music, hear it, and export real `.sid` files.

- Tracker UI (React + Tauri): 3 voices, order list, instrument editor (ADSR, waveforms, pulse width, wave table), filter, undo/redo, piano keyboard, oscilloscope, light/dark theme
- Realtime audio: the same Rust engine + reSID, compiled to WebAssembly, running in an AudioWorklet
- Export `.sid` (PSID v2, PAL/NTSC, 6581/8580) and `.prg` with an own 6502 player; verified register-for-register against the engine in a 6502 emulator and played back by the independent `sidplayfp`
- Project files (`.sidb`, JSON)

Planned: pulse/filter tables and filter changes mid-song, SID-Wizard `.swm` import, MIDI input.

## Layout
- `core/` – Rust: song model, playback engine, 6502 player (`core/player`), export, reSID rendering, CLI
- `wasm/` – WebAssembly bindings of the core
- `app/` – web UI (Vite + React + TypeScript), end-to-end tests in `app/e2e`
- `src-tauri/` – desktop shell (Windows / macOS / Linux)

## Develop
```
cargo test --workspace                 # core tests (needs 64tass for the player check; sidplayfp optional)
cd app && npm ci && npm run wasm       # needs: rustup target add wasm32-unknown-unknown; cargo install wasm-bindgen-cli --version 0.2.100
npm run dev                            # UI in the browser
npm run test:e2e                       # builds are served and driven with Playwright
npm run tauri -- dev                   # desktop app (needs the Tauri prerequisites)
```

## Command line
```
cargo run --release -p sidcore --bin sidbloom -- demo demo.sidb
cargo run --release -p sidcore --bin sidbloom -- render demo.sidb demo.wav 8
cargo run --release -p sidcore --bin sidbloom -- export-sid demo.sidb demo.sid   # plays on a real C64 / any SID player
```

License: GPL-3.0-or-later (see `LICENSE`, `NOTICE.md`).
