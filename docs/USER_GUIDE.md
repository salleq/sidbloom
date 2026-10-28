# SIDbloom user guide

SIDbloom is a tracker for three-voice C64 SID music. You write notes into patterns, shape sounds with instruments,
and export real `.sid` files that play on a C64 or in any SID player.

## 1. Your first tune in two minutes
1. Start SIDbloom. A small demo song is loaded; press **Space** to hear it, **Space** again to stop.
2. Click **New** for an empty song.
3. Click a cell in **Voice 1**. Press **Z S X D C V G B H N J M** (a piano on the bottom keyboard row; **Q W E R T Y U**
   continue one octave higher). Each key writes a note and moves down one row; you hear it as you type.
4. Press **Space** to play from the top, **Enter** to play from the cursor.
5. **Export .sid** writes the file. That's it.

## 2. The screen
- **Top bar:** New / Open / Save / Import SW…, undo / redo, play / stop, speed, PAL or NTSC, SID chip (6581 or 8580),
  export buttons, theme.
- **Pattern editor (centre):** three voices. Each cell is `note  instrument  effect  parameter`:
  `A-4 01 4 23` = note A-4, instrument 01, effect 4 (vibrato) with parameter 23.
  `===` is *note off* (release the key). Row numbers are hexadecimal.
- **Right side:** song order, the instrument editor, chords, the filter, and a key reference.
- **Bottom:** status, a scope showing the sound, and a clickable piano.

## 3. Writing patterns
| Key | Does |
|---|---|
| Arrows, Tab / Shift+Tab | Move (Tab changes voice) |
| PgUp / PgDn | 16 rows up / down |
| `Z S X D C …` | Enter a note in the note column |
| `1` or `` ` `` | Note off |
| `0–9 A–F` | Type hex in the instrument / effect columns |
| Del / Backspace | Clear the cell |
| Insert / Shift+Backspace | Insert / delete a row (the rest of the pattern shifts) |
| `[` `]` | Octave down / up |
| `-` `=` | Previous / next instrument |
| Ctrl+Z / Ctrl+Y | Undo / redo (200 steps) |
| Ctrl+S / Ctrl+O | Save / open a `.sidb` project |

**Step** (status bar) sets how many rows the cursor moves after entering a note; 0 keeps it in place.

## 4. Song order
A song is a list of *positions*; each position names one pattern per voice. Patterns are reused, so one drum pattern
can serve a whole song. Use **+ Position / − Position** to extend the song, ◀ ▶ to pick a different pattern for the
current voice at the current position, **New** / **Duplicate** to make patterns, and **Rows** to change a pattern's
length (1–64). Playback loops when every voice reaches the end of its list.

## 5. Instruments
- **ADSR** shapes the volume of each note; the graph shows it.
- **Pulse width** sets the timbre of the pulse waveform. **Pulse sweep** moves it every frame between a minimum and
  maximum, which gives the classic moving "chorus" sound.
- **Waveform table:** one step per video frame. Each step chooses waveforms (Tri / Saw / Pulse / Noise, with Ring and
  Sync modifiers) and a semitone offset. Use it for drum hits (noise → pulse with falling pitch) or fast arpeggios.
  ↺ makes the table loop back to a step; without it the last step is held.
- **Chord:** if set, the instrument steps through that chord's notes, one per frame (see below).
- **♪ Test** plays the instrument.

## 6. Effects
Type the effect number in the third column and two hex digits in the parameter columns.

| Fx | Meaning | Parameter |
|---|---|---|
| 0xy | Arpeggio | x, y semitones above the note |
| 1xx | Slide up | frequency units per frame |
| 2xx | Slide down | frequency units per frame |
| 3xx | Chord | chord number (chord instruments only) |
| 4xy | Vibrato | x speed, y depth |
| 8xx | Filter cutoff | 00–FF |
| 9xy | Filter resonance and routing | x resonance, y bits = voices 3 2 1 |
| Axy | Filter mode and volume | x = 1 low-pass, 2 band-pass, 4 high-pass (add them); y = volume |
| Bxx | Filter cutoff sweep | signed change per frame (FE = −2); stays until changed |
| Fxx | Speed | frames per row |

Speed 6 at PAL is about 125 BPM at four rows per beat.

## 7. Chords
A chord is a short list of semitone offsets, for example `0 4 7` (a major chord). An instrument with a chord plays
the offsets one after another, one per frame, which the ear hears as a chord. Add chords in the **Chords** panel
(write the offsets separated by spaces) and choose one in the instrument's **Chord** menu. **hold** keeps the
last offset instead of looping.

## 8. Filter
The SID has one filter shared by all voices. Choose which voices go through it, the mode, cutoff and resonance in the
**Filter & volume** panel, or change them while the song plays with effects 8–B.

## 9. Files
- **Save / Open:** `.sidb` project files (readable JSON).
- **Export .sid:** the standard format, plays in VICE, SIDPlay, Winamp plug-ins, a SID player on a real C64 or
  SIDs on a SD card. SID 6581 / 8580 and PAL / NTSC are written into the file.
- **.prg:** a program that starts itself: `LOAD"*",8,1` then `RUN`.
- **Import SW…:** opens a SID-Wizard 1.x module (`.swm`, or `.prg` / `.p00` of one). Anything this tracker cannot
  represent is approximated and listed in a report above the editor.

## 10. Limits (set by the C64 player)
64 instruments · 128 patterns · 64 rows per pattern · 128 wave-table steps · 32 chords of up to 16 notes · 255
positions. Export tells you if something is over the limit. The player is small: it uses about a quarter of a frame.
