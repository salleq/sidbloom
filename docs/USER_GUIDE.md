# SIDbloom user guide

SIDbloom is a tracker for three-voice C64 SID music. You write notes into patterns, shape sounds with instruments,
and export real `.sid` files that play on a C64 or in any SID player.

## 1. Your first tune in two minutes
1. Start SIDbloom. A small demo song is loaded; press **Space** to hear it, **Space** again to stop.
2. Click **New** for an empty song.
3. Click a cell in **Voice 1**. Press **Z S X D C V G B H N J M** (a piano on the bottom keyboard row; **Q W E R T Y U**
   continue one octave higher). Each key writes a note and moves down one row; you hear it as you type.
4. Press **Space** to loop the current pattern, **Shift+Space** to play the whole song from the top, **Enter** to play the song from the cursor.
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
| `<` `>` · F1 F2 · `[` `]` | Octave down / up (also the − + buttons) |
| `-` `=` | Previous / next instrument |
| Drag with the mouse, or Shift+arrows | Select cells (several rows and voices) |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy, cut, paste the selection at the cursor (a single cell when nothing is selected); Del clears the selection |
| Ctrl+Z / Ctrl+Y | Undo / redo (200 steps) |
| Ctrl+S / Ctrl+Shift+S / Ctrl+O | Save (overwrites the current file) / save as / open a `.sidb` project |

New, Open and the imports ask before they replace a song with unsaved changes. A new song gives each voice its own pattern;
when several voices show the same pattern number in the order list they share it, and editing one changes the others.

**Step** (status bar) sets how many rows the cursor moves after entering a note; 0 keeps it in place.

### Playing with a MIDI keyboard
Connect the keyboard, open the **MIDI** menu in the top bar and pick it (in a web browser the first click asks for permission to use MIDI devices). Keys now sound with the selected instrument and, while **Rec** is on, are written into the pattern at the
cursor exactly like the computer-keyboard piano (then the cursor moves by **Step** rows). Turn **Rec** off to just play.
Middle C (MIDI 60) is C-4. Notes outside C-0 … B-7 are ignored; velocity is ignored. While a song is playing, the
keyboard only sounds. On macOS the first start may ask for permission to use MIDI devices.

## 4. Song order
A song is a list of *positions*. Use **+ Position / − Position** to extend the song, ◀ ▶ to pick a different pattern at the
current position, **New** / **Duplicate** to make patterns, and **Rows** to change a pattern's length (1–64). Playback
loops when every voice reaches the end of its list.

**Linked patterns** (the checkbox in the order panel; on for new songs) work like a `.mod` tracker: one pattern number
covers all three voices, so **New**, **Duplicate**, ◀ ▶ and **Rows** act on every voice at once. Together with
**Ctrl+A** (select the whole pattern) and **Ctrl+[ / Ctrl+]** (transpose the selection by a semitone; add Shift for an
octave) you can work on a whole bar at a time. The SID file you export is the same either way; the switch only changes
how you edit.

With linked patterns off, every voice has its own pattern number at each position, so patterns can be mixed freely (one
drum pattern can serve a whole song, and voices may have patterns of different lengths). Switching linked patterns on
copies each position's three patterns into one combined pattern. That is only offered when the voices have the same number of
positions and equal pattern lengths at every position (otherwise the player's timing would change), and it can be undone.
Songs imported from SID-Wizard or MIDI start unlinked.

## 5. Instruments
- **ADSR** shapes the volume of each note; the graph shows it.
- **Pulse width** sets the timbre of the pulse waveform. **Pulse sweep** moves it every frame between a minimum and
  maximum, which gives the classic moving "chorus" sound.
- **Waveform table:** one step per video frame. Each step chooses waveforms (Tri / Saw / Pulse / Noise, with Ring and
  Sync modifiers) and a semitone offset. Use it for drum hits (noise → pulse with falling pitch) or fast arpeggios.
  ↺ makes the table loop back to a step; without it the last step is held.
- **Chord:** if set, the instrument steps through that chord's notes, one per frame (see below).
- **♪ Test** plays the instrument.
- **Add from library…** adds a ready-made instrument: drums (kick, snares, clap, hihats, tom, crash, ride), basses, leads, a
  pad and arpeggio chords. Drums sound best at the pitch they were made for (kicks around C-2, snares around C-3, hihats high up),
  so try other notes if one sounds off.
- **Export / Import** save one instrument to a `.sidi` file and add one from a file, to share sounds between songs.

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
- **Open a `.sid` / `.prg` you exported:** SIDbloom stores your project inside the file (after the music data; players ignore
  it), so **Open** can bring an exported tune back for editing. Files not made by SIDbloom, and exports from versions
  older than this feature, cannot be opened this way. Very large songs may be too big to embed; export tells you.
- **Export .sid:** the standard format, plays in VICE, SIDPlay, Winamp plug-ins, a SID player on a real C64 or
  SIDs on a SD card. SID 6581 / 8580 and PAL / NTSC are written into the file.
- **.prg:** a program that starts itself: `LOAD"*",8,1` then `RUN`.
- **Import MIDI…:** opens a standard MIDI file (`.mid`). A dialog lists its parts (tracks or channels) and lets you pick one
  for each of the three voices; the busiest three are preselected. A SID voice plays one note at a time, so chords are reduced
  to their highest note, the drum channel becomes a noise instrument, and timing is rounded to four rows per beat. Only the first
  tempo is used. What was changed is listed in the report above the editor.
- **Import SW…:** opens a SID-Wizard 1.x module (`.swm`, or `.prg` / `.p00` of one). Anything this tracker cannot
  represent is approximated and listed in a report above the editor.

## 10. Limits (set by the C64 player)
64 instruments · 128 patterns · 64 rows per pattern · 128 wave-table steps · 32 chords of up to 16 notes · 255
positions. Export tells you if something is over the limit. The player is small: it uses about a quarter of a frame.
