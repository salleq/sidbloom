//! Standard MIDI File (`.mid`) importer: turns up to three parts (a track, or one channel of a track)
//! into the three SID voices. SID voices are monophonic, so chords are reduced to their highest note.
use crate::model::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum MidiError {
    NotMidi,
    Unsupported(&'static str),
    Truncated,
    TooLarge,
    NoNotes,
}

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiError::NotMidi => write!(f, "not a MIDI file"),
            MidiError::Unsupported(w) => write!(f, "unsupported MIDI file: {w}"),
            MidiError::Truncated => write!(f, "the MIDI file is truncated or damaged"),
            MidiError::TooLarge => write!(f, "the MIDI file is too large"),
            MidiError::NoNotes => write!(f, "the MIDI file contains no notes"),
        }
    }
}
impl std::error::Error for MidiError {}

const MAX_FILE: usize = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 2_000_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MidiPart {
    pub track: usize,
    pub channel: u8,
    pub name: String,
    pub notes: usize,
    pub min: u8,
    pub max: u8,
    pub drum: bool,
}

#[derive(Clone, Copy, Debug)]
struct Note {
    start: u64,
    end: u64,
    key: u8,
}

struct Parsed {
    division: u32,
    tempos: Vec<(u64, u32)>, // (tick, microseconds per quarter note)
    parts: Vec<MidiPart>,
    notes: Vec<Vec<Note>>, // per part
    title: String,
    last_tick: u64,
}

pub struct MidiImport {
    pub song: Song,
    pub warnings: Vec<String>,
}

fn varlen(d: &[u8], pos: &mut usize) -> Result<u32, MidiError> {
    let mut v = 0u32;
    for _ in 0..4 {
        let b = *d.get(*pos).ok_or(MidiError::Truncated)?;
        *pos += 1;
        v = (v << 7) | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(MidiError::Truncated)
}

fn text(b: &[u8]) -> String {
    b.iter().map(|&c| if (0x20..0x7F).contains(&c) { c as char } else { ' ' }).collect::<String>().trim().to_string()
}

fn parse(d: &[u8]) -> Result<Parsed, MidiError> {
    if d.len() > MAX_FILE {
        return Err(MidiError::TooLarge);
    }
    // RIFF-wrapped (.rmi) files carry the SMF after a 20 byte header
    let d = if d.starts_with(b"RIFF") && d.len() > 20 && &d[8..12] == b"RMID" { &d[20..] } else { d };
    if d.len() < 14 || &d[0..4] != b"MThd" {
        return Err(MidiError::NotMidi);
    }
    let hlen = u32::from_be_bytes(d[4..8].try_into().unwrap()) as usize;
    if hlen < 6 || 8 + hlen > d.len() {
        return Err(MidiError::Truncated);
    }
    let ntrks = u16::from_be_bytes([d[10], d[11]]) as usize;
    let division = u16::from_be_bytes([d[12], d[13]]);
    if division & 0x8000 != 0 {
        return Err(MidiError::Unsupported("SMPTE time division"));
    }
    if division == 0 {
        return Err(MidiError::Unsupported("zero ticks per quarter note"));
    }
    let mut pos = 8 + hlen;

    let mut tempos: Vec<(u64, u32)> = vec![];
    let mut parts: Vec<MidiPart> = vec![];
    let mut notes: Vec<Vec<Note>> = vec![];
    let mut title = String::new();
    let mut last_tick = 0u64;
    let mut events = 0usize;

    for track in 0..ntrks {
        // skip unknown chunks
        let (len, body) = loop {
            if pos + 8 > d.len() {
                return if track == 0 { Err(MidiError::Truncated) } else { Ok(finish(division, tempos, parts, notes, title, last_tick)) };
            }
            let id = &d[pos..pos + 4];
            let len = u32::from_be_bytes(d[pos + 4..pos + 8].try_into().unwrap()) as usize;
            let body_start = pos + 8;
            pos = body_start.saturating_add(len);
            if id == b"MTrk" {
                break (len, &d[body_start..body_start.saturating_add(len).min(d.len())]);
            }
        };
        let _ = len;
        let mut p = 0usize;
        let mut tick = 0u64;
        let mut running = 0u8;
        let mut track_name = String::new();
        let mut open: HashMap<(u8, u8), u64> = HashMap::new(); // (channel, key) -> start
        let mut chan_notes: HashMap<u8, Vec<Note>> = HashMap::new();
        while p < body.len() {
            events += 1;
            if events > MAX_EVENTS {
                return Err(MidiError::TooLarge);
            }
            tick += varlen(body, &mut p)? as u64;
            let first = *body.get(p).ok_or(MidiError::Truncated)?;
            let status = if first & 0x80 != 0 {
                p += 1;
                if first < 0xF0 {
                    running = first;
                }
                first
            } else {
                if running == 0 {
                    return Err(MidiError::Truncated);
                }
                running
            };
            match status {
                0xFF => {
                    let kind = *body.get(p).ok_or(MidiError::Truncated)?;
                    p += 1;
                    let l = varlen(body, &mut p)? as usize;
                    let data = body.get(p..p + l).ok_or(MidiError::Truncated)?;
                    p += l;
                    match kind {
                        0x03 if track_name.is_empty() => track_name = text(data),
                        0x51 if l == 3 => tempos.push((tick, u32::from_be_bytes([0, data[0], data[1], data[2]]))),
                        0x2F => break,
                        _ => {}
                    }
                }
                0xF0 | 0xF7 => {
                    let l = varlen(body, &mut p)? as usize;
                    p = p.checked_add(l).filter(|&e| e <= body.len()).ok_or(MidiError::Truncated)?;
                }
                s if s >= 0xF8 => {}
                s => {
                    let ch = s & 0x0F;
                    let n_data = if (0xC0..0xE0).contains(&(s & 0xF0)) { 1 } else { 2 };
                    let args = body.get(p..p + n_data).ok_or(MidiError::Truncated)?;
                    p += n_data;
                    let (kind, key, vel) = (s & 0xF0, args[0] & 0x7F, args.get(1).copied().unwrap_or(0) & 0x7F);
                    let note_on = kind == 0x90 && vel > 0;
                    let note_off = kind == 0x80 || (kind == 0x90 && vel == 0);
                    if note_on {
                        if let Some(start) = open.insert((ch, key), tick) {
                            chan_notes.entry(ch).or_default().push(Note { start, end: tick, key });
                        }
                    } else if note_off {
                        if let Some(start) = open.remove(&(ch, key)) {
                            chan_notes.entry(ch).or_default().push(Note { start, end: tick, key });
                        }
                    }
                }
            }
        }
        last_tick = last_tick.max(tick);
        for ((ch, key), start) in open {
            chan_notes.entry(ch).or_default().push(Note { start, end: tick.max(start + 1), key });
        }
        let mut chans: Vec<_> = chan_notes.into_iter().collect();
        chans.sort_by_key(|(c, _)| *c);
        let multi = chans.len() > 1;
        for (ch, mut ns) in chans {
            ns.sort_by_key(|n| (n.start, n.key));
            let (min, max) = (ns.iter().map(|n| n.key).min().unwrap_or(0), ns.iter().map(|n| n.key).max().unwrap_or(0));
            let name = match (track_name.is_empty(), multi || ch == 9) {
                (false, true) => format!("{} (ch {})", track_name, ch + 1),
                (false, false) => track_name.clone(),
                (true, _) => format!("Channel {}", ch + 1),
            };
            if title.is_empty() && !track_name.is_empty() && ch != 9 {
                title = track_name.clone();
            }
            parts.push(MidiPart { track, channel: ch, name, notes: ns.len(), min, max, drum: ch == 9 });
            notes.push(ns);
        }
    }
    Ok(finish(division, tempos, parts, notes, title, last_tick))
}

fn finish(division: u16, tempos: Vec<(u64, u32)>, parts: Vec<MidiPart>, notes: Vec<Vec<Note>>, title: String, last_tick: u64) -> Parsed {
    Parsed { division: division as u32, tempos, parts, notes, title, last_tick }
}

/// The importable parts (tracks / channels that contain notes) of a MIDI file.
pub fn list_parts(bytes: &[u8]) -> Result<Vec<MidiPart>, MidiError> {
    let p = parse(bytes)?;
    if p.parts.is_empty() {
        return Err(MidiError::NoNotes);
    }
    Ok(p.parts)
}

/// Picks the three parts with the most notes (kept in file order); the usual starting point for the dialog.
pub fn default_assignment(parts: &[MidiPart]) -> [Option<usize>; 3] {
    let mut idx: Vec<usize> = (0..parts.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(parts[i].notes));
    idx.truncate(3);
    idx.sort();
    let mut out = [None; 3];
    for (slot, i) in idx.into_iter().enumerate() {
        out[slot] = Some(i);
    }
    out
}

/// Noise pitch per General MIDI drum sound (SID noise changes colour with the frequency register).
fn drum_pitch(key: u8) -> u8 {
    match key {
        35 | 36 => 25,                  // kick
        38 | 39 | 40 => 50,             // snare, clap
        41 | 43 | 45 | 47 | 48 | 50 => 38, // toms
        42 | 44 | 46 | 49 | 51 | 52 | 53 | 55 | 57 | 59 => 78, // hats, cymbals
        _ => 60,
    }
}

const ROWS_PER_PATTERN: usize = 32;

/// Converts the chosen parts into a song: `assign[voice]` is an index into `list_parts`.
pub fn import_midi(bytes: &[u8], assign: [Option<usize>; 3]) -> Result<MidiImport, MidiError> {
    let p = parse(bytes)?;
    if p.parts.is_empty() {
        return Err(MidiError::NoNotes);
    }
    let mut warnings = Vec::new();

    let mut song = Song::new();
    song.title = p.title.chars().take(31).collect::<String>();
    if song.title.is_empty() {
        song.title = "MIDI import".into();
    }

    // ---- timing: four rows per beat; the frame speed is the closest whole number of frames per row
    let tempo_us = p.tempos.first().map(|t| t.1).unwrap_or(500_000).max(1);
    if p.tempos.iter().any(|t| t.1 != tempo_us) {
        warnings.push("the file changes tempo; only the first tempo was used".into());
    }
    let frames_per_row = song.frame_hz() * (tempo_us as f64 / 1e6) / 4.0;
    song.speed = frames_per_row.round().clamp(1.0, 31.0) as u8;
    let bpm = 60e6 / tempo_us as f64;
    let played_bpm = 60.0 * song.frame_hz() / (song.speed as f64 * 4.0);
    if (played_bpm - bpm).abs() / bpm > 0.04 {
        warnings.push(format!("tempo {bpm:.0} BPM was rounded to {played_bpm:.0} BPM (the speed is a whole number of frames per row)"));
    }
    let row_ticks = (p.division as f64 / 4.0).max(1.0);
    if p.division < 4 {
        warnings.push("very coarse MIDI time resolution; timing may be off".into());
    }

    // ---- instruments
    song.instruments = vec![
        {
            let mut i = Instrument::simple("MIDI lead", 0x40, 0x08, 0xA9);
            i.pulse_speed = 10;
            i.pulse_min = 0x400;
            i.pulse_max = 0xC00;
            i
        },
        Instrument::simple("MIDI bass", 0x20, 0x05, 0xA8),
        Instrument::simple("MIDI drums", 0x80, 0x00, 0x00),
    ];
    {
        let drum = &mut song.instruments[2];
        drum.attack_decay = 0x04; // instant attack, short decay
        drum.sustain_release = 0x00;
    }

    // ---- notes -> rows, per voice
    let mut voice_rows: [Vec<Cell>; 3] = [vec![], vec![], vec![]];
    let mut dropped_poly = 0usize;
    let mut shifted = 0usize;
    for (v, slot) in assign.iter().enumerate() {
        let Some(&part) = slot.as_ref() else { continue };
        let Some(info) = p.parts.get(part) else { continue };
        let drum = info.drum;
        let median = {
            let mut ks: Vec<u8> = p.notes[part].iter().map(|n| n.key).collect();
            ks.sort();
            ks.get(ks.len() / 2).copied().unwrap_or(60)
        };
        let inst = if drum { 3 } else if median < 52 { 2 } else { 1 };
        // quantize and keep one note per start row (the highest)
        let mut by_row: Vec<(usize, usize, u8)> = Vec::new(); // (start row, end row, tracker note)
        for n in &p.notes[part] {
            let start = (n.start as f64 / row_ticks).round() as usize;
            let end = ((n.end as f64 / row_ticks).round() as usize).max(start + 1);
            let key = if drum {
                drum_pitch(n.key)
            } else {
                let mut t = n.key as i32 - 11;
                while t < 1 {
                    t += 12;
                    shifted += 1;
                }
                while t > 95 {
                    t -= 12;
                    shifted += 1;
                }
                t as u8
            };
            match by_row.last_mut() {
                Some(last) if last.0 == start => {
                    dropped_poly += 1;
                    if key > last.2 {
                        *last = (start, end, key);
                    }
                }
                _ => by_row.push((start, end, key)),
            }
        }
        let total = by_row.iter().map(|b| b.1).max().unwrap_or(0);
        let rows = &mut voice_rows[v];
        rows.resize(total + 1, Cell::default());
        for (i, &(start, end, key)) in by_row.iter().enumerate() {
            rows[start] = Cell { note: key, inst, ..Default::default() };
            // a note-off where the note ends, unless the next note takes over at or before that row
            let next_start = by_row.get(i + 1).map(|b| b.0);
            let off = match next_start {
                Some(ns) if ns <= end => None,
                _ => Some(end),
            };
            if let Some(o) = off {
                if o < rows.len() && rows[o].note == 0 {
                    rows[o].note = NOTE_OFF;
                }
            }
        }
    }
    if dropped_poly > 0 {
        warnings.push(format!("{dropped_poly} notes were left out because a SID voice plays one note at a time (the highest note was kept)"));
    }
    if shifted > 0 {
        warnings.push(format!("{shifted} octave shifts were needed to fit the notes into the tracker's range"));
    }
    let used: Vec<usize> = assign.iter().flatten().copied().collect();
    let unused = p.parts.len().saturating_sub(used.len());
    if unused > 0 {
        warnings.push(format!("{unused} other parts were not imported (a SID has only three voices)"));
    }
    if p.last_tick == 0 {
        return Err(MidiError::NoNotes);
    }

    // ---- rows -> patterns and orders (identical patterns are shared)
    let total_rows = voice_rows.iter().map(|r| r.len()).max().unwrap_or(0).max(1);
    let positions = total_rows.div_ceil(ROWS_PER_PATTERN).min(crate::export::pack::MAX_ORDER);
    if positions * ROWS_PER_PATTERN < total_rows {
        warnings.push("the song is longer than the player supports; the end was cut off".into());
    }
    song.patterns.clear();
    let mut dedupe: HashMap<Vec<Cell>, u8> = HashMap::new();
    let mut orders: [Vec<u8>; 3] = [vec![], vec![], vec![]];
    let mut cut_at: Option<usize> = None;
    'outer: for pos in 0..positions {
        let mut ids = [0u8; 3];
        for v in 0..3 {
            let mut chunk: Vec<Cell> = (0..ROWS_PER_PATTERN).map(|r| voice_rows[v].get(pos * ROWS_PER_PATTERN + r).copied().unwrap_or_default()).collect();
            // trailing positions keep their full length so every voice stays in step
            if chunk.is_empty() {
                chunk = vec![Cell::default(); ROWS_PER_PATTERN];
            }
            let id = match dedupe.get(&chunk) {
                Some(&id) => id,
                None => {
                    if song.patterns.len() >= crate::export::pack::MAX_PATTERNS {
                        cut_at = Some(pos);
                        break 'outer;
                    }
                    let id = song.patterns.len() as u8;
                    song.patterns.push(Pattern { rows: chunk.clone() });
                    dedupe.insert(chunk, id);
                    id
                }
            };
            ids[v] = id;
        }
        for v in 0..3 {
            orders[v].push(ids[v]);
        }
    }
    if let Some(pos) = cut_at {
        warnings.push(format!("the player's pattern limit was reached at position {pos}; the end of the song was cut off"));
    }
    if orders[0].is_empty() {
        song.patterns.push(Pattern::new(ROWS_PER_PATTERN));
        orders = [vec![0], vec![0], vec![0]];
    }
    song.orders = orders;
    Ok(MidiImport { song, warnings })
}
