## SIDbloom 0.3.0

A modern, free and open-source tracker for three-voice C64 SID music. Write music, hear it, and export real C64 files.

### What you can do
- **Compose** in a three-voice tracker with instruments (ADSR, pulse sweep, waveform tables, chords), effects (arpeggio, slide,
  vibrato, chords, filter control) and undo/redo. Edits made during playback are heard immediately.
- **Export** `.sid` files (play on a C64, in VICE, SIDPlay…) and a self-starting `.prg` (`LOAD"*",8,1` + `RUN`).
  Your own exports **open again for editing**.
- **Import** SID-Wizard 1.x modules (`.swm`), MIDI files (`.mid`), and play notes from a **MIDI keyboard**.
- **Play** the current pattern in a loop (Space), the whole song (Shift+Space) or from the cursor (Enter); double-click a
  position in the song order to hear that pattern. Octave buttons and `<` `>` / F1 F2 keys work on a Finnish keyboard.
- **Save** overwrites the project file, **Save As…** asks for a new name. The side column is tabs instead of one long list.
- Windows, macOS and Linux, on x64 and ARM64.

### Downloads
| System | File |
|---|---|
| Windows x64 | `.msi` or `.exe` |
| Windows ARM64 | `.exe` (NSIS) |
| macOS (Apple silicon + Intel) | `.dmg` |
| Linux x64 / ARM64 | `.AppImage`, `.deb`, `.rpm` |

`SHA256SUMS.txt` lists the checksums of every file.

### Please read
- **Linux `.deb`:** the package is now called `sidbloom` (earlier test builds installed as `si-dbloom`). Remove the old one
  first: `sudo dpkg -r si-dbloom`.
- The installers are **not signed**, so the system warns on first start. macOS: right-click the app → *Open*.
  Windows: *More info* → *Run anyway*. Linux AppImage: `chmod +x` it first.
- This is a **preview**. The author has not yet listened to the Windows and macOS builds or played the exported files on a real
  C64, and MIDI keyboard input has only been tested with a simulated keyboard. Automated tests cover the engine, the 6502
  player (register-for-register against the editor), the exports (also played back in `sidplayfp`) and the interface on
  all three operating systems. Please report what does not work.
- SID-Wizard import approximates features this tracker lacks (filter programs, built-in vibrato, …) and lists them.

Full details: [CHANGELOG](https://github.com/salleq/sidbloom/blob/main/CHANGELOG.md) ·
[User guide](https://github.com/salleq/sidbloom/blob/main/docs/USER_GUIDE.md).
Licence: GPL-3.0-or-later. Uses the reSID emulation; see [NOTICE](https://github.com/salleq/sidbloom/blob/main/NOTICE.md).
