# Licensing and third-party notices

This project is free software, licensed under **GPL-3.0-or-later** (see `LICENSE`).
It is non-commercial and fully open source.

## Third-party components

| Component | License | Use |
|---|---|---|
| [resid-rs](https://github.com/binaryfields/resid-rs) (Rust port of reSID) | GPL-3.0-or-later | SID chip emulation (linked) |
| [reSID](https://github.com/daglem/reSID) by Dag Lem | GPL-2.0-or-later | Original of the above |
| [64tass](https://tass64.sourceforge.net/) | GPL-2.0-or-later | Build tool only, not distributed in the app |
| SID-Wizard by Hermit | "Public Domain" per its SourceForge project page | `.swm` import: written from its module specification (`SWM-spec.src`), user manual and converter source as format documentation; no code copied |

GPL-2.0-or-later and GPL-3.0-or-later are compatible, so combining them under GPL-3.0-or-later is fine.

## Rules for contributors
- No code copied from a project unless its license is verified here first (GPL-compatible or public domain).
- The `.swm` importer is written from the format documentation / file inspection. If any
  SID-Wizard source file is copied verbatim, first check that file's own header: the
  SourceForge project field says public domain, but bundled third-party parts may differ.
- The `.sid` format (PSID/RSID) is an open specification; exported files contain only our
  own player code plus the user's music data.

## Application (UI) dependencies

| Component | License | Use |
|---|---|---|
| [React](https://react.dev) | MIT | UI |
| [Tauri](https://tauri.app) + plugins (dialog, fs) | Apache-2.0 OR MIT | Desktop shell |
| [midir](https://github.com/Boddlnagg/midir) | MIT | MIDI keyboard input (desktop app) |
| [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) | MIT OR Apache-2.0 | Rust/WebAssembly bindings |

Apache-2.0 and MIT are compatible with GPL-3.0-or-later. Dependency licenses should be re-listed
(`cargo license`, `npx license-checker`) before each release.

## Development-only dependencies
| Component | License | Use |
|---|---|---|
| [mos6502](https://github.com/mre/mos6502) | BSD-3-Clause | 6502 CPU emulator used only in tests (not shipped) |
| Vite, TypeScript, Playwright | MIT / Apache-2.0 | Build and end-to-end tests |
| [sidplayfp](https://github.com/libsidplayfp/sidplayfp) | GPL-2.0-or-later | Independent player used in a cross-check test only |

## SID-Wizard modules in tests
`core/tests/swm_import.rs` builds a tiny module by hand. The optional test over real modules runs only when
`SIDBLOOM_SWM_DIR` points at a folder of `.swm` files; those tunes are **not** included in this repository
because their individual authors' rights are not established.
