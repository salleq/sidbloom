# Contributing to SIDbloom

SIDbloom is free software (GPL-3.0-or-later) and stays non-commercial. Bug reports, ideas and pull requests are welcome.

## Running the tests
```
cargo test --workspace            # engine, 6502 player, exports, importers
cd app && npm ci && npm run wasm  # once: build the WebAssembly module
npm run build && npm run test:e2e # user interface, driven with Playwright
```
Some tests use external tools and skip themselves when those are missing: `64tass` (the 6502 assembler, to check that
`core/player/*.bin` match their `.asm`) and `sidplayfp` (an independent SID player).
If you change `core/player/player.asm` or `stub.asm`, rebuild the binaries:
`64tass -b -o core/player/player.bin core/player/player.asm` (same for `stub`).

## The rule that matters most
**The Rust engine (`core/src/engine.rs`) and the 6502 player (`core/player/player.asm`) must produce identical SID
register writes.** A new effect or instrument feature therefore touches both, plus `pack.rs`, and is checked by
`core/tests/player_matches_engine.rs` (which also randomizes the new feature). Do not merge one without the other.

## Licensing
- Every dependency must be compatible with GPL-3.0-or-later and be listed in `NOTICE.md`.
- Do not copy code from SID-Wizard or any other project without first checking the licence of that very file.
  The SID-Wizard importer was written from its published module specification and manual.
- Do not add music (tunes, modules) to the repository unless you know the author allows it.

## Style
Match the surrounding code. Keep user-visible text clear and plain; say what an import or export could not do instead of
hiding it. Add a line to `CHANGELOG.md` under *Unreleased* for anything a user would notice.
