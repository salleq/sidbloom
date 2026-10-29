# Changelog

All notable changes to SIDbloom are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Instrument library: 23 presets (kick, snares, clap, hihats, tom, crash, ride, basses, leads, pad, arpeggios), and Import / Export of single instruments as `.sidi` files.
- Linked patterns (on for new songs, per song): one pattern number covers all three voices, like a `.mod` tracker. Existing songs can be switched when their voices line up. Also Ctrl+A to select a whole pattern and Ctrl+[ / ] to transpose the selection.
- Select cells with the mouse (drag) or Shift+arrows; copy, cut and paste them with Ctrl+C / X / V or the new buttons. Del clears the selection.
- New, Open and the imports now ask before they replace a song with unsaved changes; closing the browser tab warns too.

### Fixed
- A new song gave all three voices the same pattern, so editing one voice edited all. Each voice now has its own pattern.

## [0.1.0] — first public preview

The first version that can be used to write music from start to finish: compose in a tracker, hear it,
export real C64 files, and bring music in from SID-Wizard and MIDI.

### Composing
- Three-voice tracker: note, instrument, effect and parameter per cell, an order list shared by the voices,
  patterns of 1–64 rows, hexadecimal row numbers, undo/redo (200 steps), a piano keyboard on the computer keyboard.
- Instrument editor with an ADSR graph, pulse width and **pulse sweep**, a per-frame **waveform table** (triangle, saw,
  pulse, noise, ring, sync, pitch offset, loop point) and **chords** (per-frame semitone steps, loop or hold).
- Effects: arpeggio, slide up/down, vibrato, chord select, speed, and four filter effects
  (cutoff, resonance/routing, mode/volume, cutoff sweep) that work while the song plays.
- Filter and master volume panel; PAL/NTSC and SID 6581/8580 choice; light and dark theme.
- Real-time playback with the reSID emulation, an oscilloscope, playback from the cursor, and **edits made during
  playback are heard immediately** without restarting.

### Files
- **Export `.sid`** (PSID v2) and a **self-starting `.prg`** (`LOAD"*",8,1`, `RUN`) that use a small 6502 player of
  our own. The player produces exactly the same SID register writes as the editor's engine (checked on 300 random songs
  in a 6502 emulator), and exported files play back correctly in the independent `sidplayfp` player.
- Exported `.sid` and `.prg` files carry the project inside (after the music data, ignored by players), so
  **Open** can bring your own exported tune back for editing.
- Project files (`.sidb`, readable JSON).
- **Import SID-Wizard 1.x modules** (`.swm`, and `.prg`/`.p00` wrappers), tested on all of SID-Wizard's own demo tunes:
  patterns, orderlists with transposes, instruments (ADSR, wave-arp table, pulse program) and chords come across; what cannot
  be represented is listed in a report.
- **Import MIDI files** (`.mid`): choose which part plays on which voice; chords are reduced to their highest note,
  the drum channel becomes a noise instrument, timing is rounded to four rows per beat.
- **MIDI keyboard input**: play the selected instrument from a MIDI keyboard and record notes into the pattern at the cursor.

### Platforms
- Desktop app (Tauri) for Windows x64 and ARM64, macOS (Apple silicon and Intel, one universal app) and Linux x64 and ARM64.
- A command line tool (`sidbloom`) for rendering to WAV, exporting and importing.

### Known limitations
- The installers are **not signed**: macOS and Windows warn on first start (see the README).
- The audio of the Windows and macOS builds and the exported files on a **real C64 / SID chip** have not been
  listened to by the author yet. Automated checks cover the engine, the player, the exports and the user interface on all
  three operating systems.
- MIDI keyboard input has been tested only with a simulated keyboard, not with real hardware.
- SID-Wizard import: filter programs, built-in vibrato, absolute-pitch arpeggios, multispeed, tempo programs, extra
  subtunes and chord speed are not imported. The `$7E`/`$7F` chord end codes are interpreted as "hold" / "loop".
- MIDI import uses only the first tempo and no controllers or pitch bend.
- Limits set by the C64 player: 64 instruments, 128 patterns, 64 rows per pattern, 128 wave-table steps,
  32 chords of up to 16 notes, 255 order positions.
- Windows ARM64 is provided as an NSIS installer only (no `.msi`).

[Unreleased]: https://github.com/salleq/sidbloom/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/salleq/sidbloom/releases/tag/v0.1.0
