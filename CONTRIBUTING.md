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

## Screenshots
The images in `docs/screenshots/` come from `cd app && npm run build && npm run screenshots`. Regenerate them when the
interface changes noticeably.

## Releases
Releases are automatic. Start commit titles (or the pull request title, when squash-merging) with `fix:` for a bug fix (patch
version), `feat:` for a new feature (minor version) or `docs:` / `chore:` / `test:` for changes that need no release.
After a merge to `main`, a "release PR" appears (or updates) with the new version and changelog; merging it creates the
tag and the GitHub release, and the installers for every platform are built and attached to it (about 3 minutes).
To rebuild the installers of an existing release, run the *Release* workflow by hand with its tag.
