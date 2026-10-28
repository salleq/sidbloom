# SIDbloom

A modern, free and open-source 3-channel SID music tracker for Windows and macOS.
Compose C64 music with a clearer interface and export real, C64-compatible `.sid` files.

**Status:** early but working. You can write music, hear it, and export real `.sid` files.

- Tracker UI (React + Tauri): 3 voices, order list, instrument editor (ADSR, waveform table, pulse width + sweep, chords), filter and filter effects (cutoff, resonance, mode, sweep), undo/redo, piano keyboard, oscilloscope, light/dark theme
- Realtime audio: the same Rust engine + reSID, compiled to WebAssembly, running in an AudioWorklet; edits made during playback are heard immediately without restarting
- Export `.sid` (PSID v2, PAL/NTSC, 6581/8580) and a self-starting `.prg` (`LOAD"*",8,1` + `RUN`) with an own 6502 player; verified register-for-register against the engine in a 6502 emulator and played back by the independent `sidplayfp`
- MIDI keyboard input: play and record notes from a MIDI keyboard (desktop app via the system MIDI; browsers with Web MIDI)
- Project files (`.sidb`, JSON)
- **Import of SID-Wizard 1.x modules** (`.swm`, `.prg`, `.p00`): tested against all of SID-Wizard's own demo tunes (50+), which then export and play back identically in the 6502 player. Features this tracker lacks (filter programs, built-in vibrato, absolute-pitch arps, extra subtunes, tempo programs, chord speed) are approximated and listed in an import report

Planned: more effects. MIDI keyboard input is implemented and tested with a simulated keyboard; it has not been tried with real hardware yet.

See the **[user guide](docs/USER_GUIDE.md)**.

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
# desktop app (needs the Tauri prerequisites); run from the repo root:
cd .. && node app/node_modules/@tauri-apps/cli/tauri.js dev
```

## Command line
```
cargo run --release -p sidcore --bin sidbloom -- demo demo.sidb
cargo run --release -p sidcore --bin sidbloom -- render demo.sidb demo.wav 8
cargo run --release -p sidcore --bin sidbloom -- export-sid demo.sidb demo.sid   # plays on a real C64 / any SID player
cargo run --release -p sidcore --bin sidbloom -- import-swm tune.swm.prg tune.sidb
```

License: GPL-3.0-or-later (see `LICENSE`, `NOTICE.md`).
