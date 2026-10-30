//! SID-Wizard 1.x module (`.swm`) export, the inverse of `import::swm`. Written from SID-Wizard's published module
//! specification and user manual (SID-Wizard is public domain).
//!
//! The module is three sequences (one per voice, ending in a jump back to the start), the patterns they use,
//! instruments, the chord table and the tempo of the one subtune. Things SID-Wizard has no counterpart for are
//! left out and listed in `SwmExport::warnings`.
use crate::model::*;
use std::collections::HashMap;

/// Load address of a SID-Wizard module saved as a C64 PRG file.
pub const LOAD_ADDRESS: u16 = 0x3333;

const HEADER: usize = 64;
const INFO_POS: usize = 24;
const INFO_LEN: usize = HEADER - INFO_POS;
const NAME_LEN: usize = 8;
const PACKED_MIN: u8 = 0x70;
const PACKED_MAX: u8 = 0x77;

/// Limits of the SID-Wizard editor, which has to fit a module in its memory.
pub const MAX_PATTERNS: usize = 127;
pub const MAX_INSTRUMENTS: usize = 62;
const MAX_SEQUENCE: usize = 126; // entries before the closing jump
const MAX_PATTERN_BYTES: usize = 249;
const MAX_INSTRUMENT_BYTES: usize = 120; // the editor reserves 128 per instrument
const MAX_CHORD_TABLE: usize = 255;
const MAX_MODULE: usize = 0x9000;

#[derive(Debug, PartialEq, Eq)]
pub enum SwmExportError {
    TooManyInstruments(usize),
    TooManyPatterns(usize),
    SequenceTooLong { voice: usize, entries: usize },
    TooBig(usize),
}

impl std::fmt::Display for SwmExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwmExportError::TooManyInstruments(n) => write!(f, "SID-Wizard holds at most {MAX_INSTRUMENTS} instruments, the song has {n}"),
            SwmExportError::TooManyPatterns(n) => write!(f, "SID-Wizard holds at most {MAX_PATTERNS} different patterns, the song needs {n}"),
            SwmExportError::SequenceTooLong { voice, entries } => write!(f, "voice {voice} has {entries} song positions; SID-Wizard allows {MAX_SEQUENCE}"),
            SwmExportError::TooBig(n) => write!(f, "the module would be {n} bytes, too big for SID-Wizard's memory"),
        }
    }
}
impl std::error::Error for SwmExportError {}

pub struct SwmExport {
    /// The module itself (starts with `SWM1`).
    pub bytes: Vec<u8>,
    /// Things that could not be represented exactly.
    pub warnings: Vec<String>,
}

impl SwmExport {
    /// The module as a C64 PRG file (load address `$3333`), which is how SID-Wizard saves it.
    pub fn to_prg(&self) -> Vec<u8> {
        let mut v = LOAD_ADDRESS.to_le_bytes().to_vec();
        v.extend_from_slice(&self.bytes);
        v
    }
}

#[derive(Default)]
struct Stats {
    arpeggio: usize,
    filter_sweep: usize,
    mode_vol: usize,
    chord_dropped: usize,
    silent_wave: usize,
    low_semis: usize,
    wave_cut: usize,
    pulse_simplified: usize,
    shared_filter: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Active {
    None,
    Slide(u8),
    Vibrato,
}

pub fn export_swm(song: &Song) -> Result<SwmExport, SwmExportError> {
    if song.instruments.len() > MAX_INSTRUMENTS {
        return Err(SwmExportError::TooManyInstruments(song.instruments.len()));
    }
    let n_inst = song.instruments.len();
    let mut st = Stats::default();
    let mut warnings = Vec::new();

    // ---- chords: notes then $7F (loops) or $7E (hold)
    let mut chord_bytes: Vec<u8> = Vec::new();
    let mut chords_kept = 0;
    for c in &song.chords {
        let notes: Vec<u8> = if c.notes.is_empty() { vec![0] } else { c.notes.iter().map(|&n| n.clamp(-127, 125) as u8).collect() };
        if chord_bytes.len() + notes.len() + 1 > MAX_CHORD_TABLE {
            break;
        }
        chord_bytes.extend_from_slice(&notes);
        chord_bytes.push(if c.hold { 0x7E } else { 0x7F });
        chords_kept += 1;
    }
    if chords_kept < song.chords.len() {
        warnings.push(format!("only the first {chords_kept} of {} chords fit into SID-Wizard's chord table", song.chords.len()));
    }

    // ---- patterns, shared between voices when identical; long ones are split into consecutive patterns
    let mut table: Vec<(Vec<u8>, usize)> = Vec::new(); // (unpacked bytes ending in $FF, rows)
    let mut known: HashMap<(Vec<u8>, usize), u8> = HashMap::new();
    let mut by_position: HashMap<Option<usize>, Vec<u8>> = HashMap::new(); // our pattern -> SID-Wizard pattern numbers
    let mut sequences: Vec<Vec<u8>> = Vec::new();
    let silent = Pattern::new(32);
    for ch in 0..CHANNELS {
        let mut seq: Vec<u8> = Vec::new();
        if ch == 0 && song.filter.volume < 15 {
            seq.push(0xA0 | song.filter.volume); // main volume
        }
        let order: Vec<Option<usize>> = if song.orders[ch].is_empty() { vec![None] } else { song.orders[ch].iter().map(|&i| Some(i as usize)).collect() };
        for pos in order {
            if !by_position.contains_key(&pos) {
                let pat = pos.and_then(|i| song.patterns.get(i)).unwrap_or(&silent);
                let mut ids = Vec::new();
                for chunk in encode_pattern(pat, n_inst, chords_kept, &mut st) {
                    let id = match known.get(&chunk) {
                        Some(&id) => id,
                        None => {
                            if table.len() >= MAX_PATTERNS {
                                return Err(SwmExportError::TooManyPatterns(table.len() + 1));
                            }
                            table.push(chunk.clone());
                            known.insert(chunk, table.len() as u8);
                            table.len() as u8
                        }
                    };
                    ids.push(id);
                }
                by_position.insert(pos, ids);
            }
            seq.extend_from_slice(&by_position[&pos]);
        }
        if seq.len() > MAX_SEQUENCE {
            return Err(SwmExportError::SequenceTooLong { voice: ch + 1, entries: seq.len() });
        }
        seq.extend_from_slice(&[0xFF, 0x00]); // jump back to the start
        sequences.push(seq);
    }

    // ---- instruments; those used on a voice that goes through the filter get the song's filter setting as a program
    let mut filtered = vec![false; n_inst];
    let mut unfiltered = vec![false; n_inst];
    for ch in 0..CHANNELS {
        let routed = song.filter.routing & (1 << ch) != 0;
        for &idx in &song.orders[ch] {
            for c in song.patterns.get(idx as usize).map_or(&[][..], |p| &p.rows[..]) {
                if c.inst != 0 && (c.inst as usize) <= n_inst {
                    if routed { filtered[c.inst as usize - 1] = true; } else { unfiltered[c.inst as usize - 1] = true; }
                }
            }
        }
    }
    let f = &song.filter;
    let use_filter = f.mode & 0x70 != 0 && f.routing & 7 != 0;
    let mut instruments: Vec<(Vec<u8>, [u8; NAME_LEN])> = Vec::new();
    for (i, ins) in song.instruments.iter().enumerate() {
        let program = (use_filter && filtered[i]).then(|| [0x80 | (f.mode & 0x70) | (f.resonance & 0x0F), (f.cutoff >> 3).min(255) as u8, 0u8]);
        if program.is_some() && unfiltered[i] {
            st.shared_filter += 1;
        }
        instruments.push((encode_instrument(ins, song, chords_kept, program, &mut st), name_bytes(&ins.name)));
    }

    // ---- assemble: header, sequences, patterns, instruments, chords, tempo programs (none), subtune tempos
    let mut m = vec![0u8; HEADER];
    m[..4].copy_from_slice(b"SWM1");
    m[4] = 1; // frame speed
    m[5] = 4; // highlight step
    m[6] = 1; // auto-advance
    m[7] = 1; // configuration bits
    m[8..11].fill(0xFF); // all voices on
    m[11] = 32; // default pattern length
    m[12] = CHANNELS as u8;
    m[13] = table.len() as u8;
    m[14] = instruments.len() as u8;
    m[15] = chord_bytes.len() as u8;
    m[16] = 0;
    m[INFO_POS..].copy_from_slice(&info_text(song));
    for seq in &sequences {
        m.extend_from_slice(seq);
        m.push(seq.len() as u8);
    }
    for (bytes, rows) in &table {
        m.extend_from_slice(&pack(bytes));
        m.pop(); // the $FF terminator is replaced by the unpacked size
        m.push(bytes.len() as u8);
        m.push(*rows as u8);
    }
    for (data, name) in &instruments {
        m.extend_from_slice(data);
        m.push(data.len() as u8); // replaces the filter program's $FF
        m.extend_from_slice(name);
    }
    m.extend_from_slice(&chord_bytes);
    m.extend_from_slice(&[song.speed.clamp(1, 0x7F) | 0x80, 0x84]); // single tempo of the one subtune
    if m.len() > MAX_MODULE {
        return Err(SwmExportError::TooBig(m.len()));
    }

    // ---- what was left out
    let items = [
        (st.arpeggio, "arpeggio effects (0xy) were left out; SID-Wizard arpeggios are chords chosen with effect 3xx"),
        (st.filter_sweep, "filter sweep effects (Bxx) were left out"),
        (st.mode_vol, "filter mode + volume effects (Axx) were exported as the filter mode only"),
        (st.chord_dropped, "chord selections of chords that did not fit the chord table were left out"),
        (st.silent_wave, "wave steps without a waveform were exported as a muted triangle"),
        (st.low_semis, "wave steps with a pitch shift out of range were limited"),
        (st.wave_cut, "instruments have waveform tables that were cut to fit SID-Wizard's instrument size"),
        (st.pulse_simplified, "instruments have pulse sweeps that were simplified to fit SID-Wizard's instrument size"),
        (st.shared_filter, "instruments are used on both filtered and unfiltered voices; they are filtered on all"),
    ];
    for (n, text) in items {
        if n > 0 {
            warnings.push(format!("{n} {text}"));
        }
    }
    if !song.released.trim().is_empty() {
        warnings.push("the 'Released' text has no place in a SID-Wizard module".into());
    }
    if use_filter && !filtered.iter().any(|&x| x) {
        warnings.push("the song's filter was not exported: no voice that goes through it plays an instrument".into());
    }
    if !song.pal {
        warnings.push("NTSC is not stored in a SID-Wizard module; it plays at the speed of the machine it runs on".into());
    }
    Ok(SwmExport { bytes: m, warnings })
}

/// `Author:Title` in capitals, 40 characters wide (SID-Wizard splits it at the colon when it makes a `.sid`).
fn info_text(song: &Song) -> [u8; INFO_LEN] {
    let author = song.author.trim();
    let text = if author.is_empty() { song.title.trim().to_string() } else { format!("{author}:{}", song.title.trim()) };
    let mut out = [b' '; INFO_LEN];
    for (o, c) in out.iter_mut().zip(text.chars()) {
        *o = if c.is_ascii_graphic() || c == ' ' { c.to_ascii_uppercase() as u8 } else { b' ' };
    }
    out
}

fn name_bytes(name: &str) -> [u8; NAME_LEN] {
    let mut out = [b' '; NAME_LEN];
    for (o, c) in out.iter_mut().zip(name.chars()) {
        *o = if c.is_ascii_graphic() { c.to_ascii_uppercase() as u8 } else { b' ' };
    }
    out
}

/// Run-length coding of empty rows: the 6502 packer's algorithm. A run of zeros is one plain zero, then
/// `$70..$77` bytes standing for 2..9 more.
fn pack(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut j = 0;
    while j < raw.len() {
        let b = raw[j];
        if b != 0 || j == 0 || raw[j - 1] != 0 {
            out.push(b);
        } else {
            let mut count = PACKED_MIN - 1;
            while raw[j + 1] == 0 {
                count += 1;
                j += 1;
                if count >= PACKED_MAX {
                    break;
                }
            }
            out.push(if count == PACKED_MIN - 1 { 0 } else { count });
        }
        j += 1;
    }
    out
}

/// A pattern as one or more chunks of (unpacked bytes ending in `$FF`, rows), each small enough for SID-Wizard.
fn encode_pattern(p: &Pattern, n_inst: usize, chords: usize, st: &mut Stats) -> Vec<(Vec<u8>, usize)> {
    let mut active = Active::None;
    let rows: Vec<Vec<u8>> = p.rows.iter().map(|c| encode_row(c, n_inst, chords, &mut active, st)).collect();
    let mut chunks = Vec::new();
    let (mut cur, mut n) = (Vec::new(), 0);
    for r in rows {
        if cur.len() + r.len() + 1 > MAX_PATTERN_BYTES {
            cur.push(0xFF);
            chunks.push((std::mem::take(&mut cur), n));
            n = 0;
        }
        cur.extend_from_slice(&r);
        n += 1;
    }
    if n > 0 || chunks.is_empty() {
        cur.push(0xFF);
        chunks.push((cur, n));
    }
    chunks
}

fn swap_nibbles(v: u8) -> u8 { v << 4 | v >> 4 }

fn encode_row(c: &Cell, n_inst: usize, chords: usize, active: &mut Active, st: &mut Stats) -> Vec<u8> {
    let new_note = (1..=95).contains(&c.note);
    let mut note = match c.note {
        1..=95 => c.note,
        NOTE_OFF => 0x7E,
        _ => 0,
    };
    if new_note {
        *active = Active::None; // a note start ends slides and vibrato in SID-Wizard
    }
    let inst = if c.inst != 0 && (c.inst as usize) <= n_inst { c.inst } else { 0 };
    // (effect byte, value) - effects below $20 carry a value byte, the others are one byte
    let mut fx: Option<(u8, Option<u8>)> = match c.fx {
        fx::SLIDE_UP => { *active = Active::Slide(1); Some((1, Some(c.fx_param))) }
        fx::SLIDE_DOWN => { *active = Active::Slide(2); Some((2, Some(c.fx_param))) }
        fx::CHORD if (c.fx_param as usize) <= chords => Some((7, Some(c.fx_param))),
        fx::CHORD => { st.chord_dropped += 1; None }
        fx::VIBRATO => { *active = Active::Vibrato; Some((8, Some(swap_nibbles(c.fx_param)))) }
        fx::FILTER_CUTOFF => Some((0x0F, Some(c.fx_param))),
        fx::FILTER_RES_ROUTE => Some((0x1F, Some(c.fx_param))),
        fx::FILTER_MODE_VOL => {
            let mode = c.fx_param & 0x70;
            if mode == 0 {
                Some((0xA0 | (c.fx_param & 0x0F), None))
            } else {
                if c.fx_param & 0x0F != 0x0F { st.mode_vol += 1; }
                Some((0xB0 | (0x08 | mode >> 4), None))
            }
        }
        fx::FILTER_SWEEP => { st.filter_sweep += 1; None }
        fx::SET_SPEED if c.fx_param > 0 => Some((0x10, Some(c.fx_param))),
        fx::ARPEGGIO if c.fx_param != 0 => { st.arpeggio += 1; None }
        _ => None,
    };
    if fx.is_none() && !new_note {
        // SID-Wizard slides and vibrato go on until stopped; here they last one row
        match std::mem::replace(active, Active::None) {
            Active::Slide(code) => fx = Some((code, Some(0))),
            Active::Vibrato => fx = Some((8, Some(0))),
            Active::None => {}
        }
    }
    let mut inst_byte = inst;
    if fx.is_some() {
        inst_byte |= 0x80;
    }
    if inst_byte != 0 {
        note |= 0x80;
    }
    let mut out = vec![note];
    if inst_byte != 0 {
        out.push(inst_byte);
    }
    if let Some((code, val)) = fx {
        out.push(code);
        out.extend(val);
    }
    out
}

struct Token {
    step: usize,
    wf: u8,
    arp: u8,
    repeats: usize,
}

fn encode_instrument(ins: &Instrument, song: &Song, chords: usize, filter: Option<[u8; 3]>, st: &mut Stats) -> Vec<u8> {
    // ---- waveform / arpeggio table: one row per run of equal steps, `repeats` more frames in a following row
    let default_step = [WaveStep { wave: 0x40, semis: 0 }];
    let steps: &[WaveStep] = if ins.wave_table.is_empty() { &default_step } else { &ins.wave_table };
    let loop_step = ins.wave_loop.map(|l| l as usize).filter(|&l| l < steps.len());
    let chord = ins.default_chord != 0 && (ins.default_chord as usize) <= chords.min(song.chords.len());
    // The row that starts the chord has no pitch column, and steps below -32 semitones do not fit it: the
    // instrument's octave shift takes the pitch of the chord row, or the part of the low steps that does not fit.
    let lowest = steps.iter().map(|s| s.semis as i32).min().unwrap_or(0);
    let octave: i32 = if chord { steps[steps.len() - 1].semis as i32 } else if lowest < -32 { -12 * ((-32 - lowest + 11) / 12) } else { 0 };
    let mut tokens: Vec<Token> = Vec::new();
    for (k, s) in steps.iter().enumerate() {
        let mut wf = s.wave | 1;
        if wf < 0x10 {
            wf |= 0x18;
            st.silent_wave += 1;
        }
        if wf >= 0xFE { wf = 0xFD; }
        let semis = s.semis as i32 - octave;
        if !(-32..=95).contains(&semis) { st.low_semis += 1; }
        let arp = match semis.clamp(-32, 95) { 0 => 0, v @ 1..=95 => v as u8, v => v as i8 as u8 };
        let last = k + 1 == steps.len();
        match tokens.last_mut() {
            Some(t) if t.wf == wf && t.arp == arp && t.repeats < 15 && loop_step != Some(k) && !(chord && last) => t.repeats += 1,
            _ => tokens.push(Token { step: k, wf, arp, repeats: 0 }),
        }
    }
    let pulse = pulse_program(ins, st);
    let filt_rows = filter.is_some() as usize;
    // drop trailing rows until the instrument fits
    let wave_len = |n: usize| tokens[..n].iter().map(|t| 1 + (t.repeats > 0) as usize).sum::<usize>();
    let jump_fits = |n: usize| loop_step.is_some_and(|l| tokens[..n].iter().any(|t| t.step == l));
    let size = |n: usize| 16 + (wave_len(n) + jump_fits(n) as usize) * 3 + 1 + pulse.len() * 3 + 1 + filt_rows * 3;
    let mut keep = tokens.len();
    while keep > 1 && size(keep) > MAX_INSTRUMENT_BYTES {
        keep -= 1;
    }
    if keep < tokens.len() {
        st.wave_cut += 1;
    }

    let mut wave: Vec<[u8; 3]> = Vec::new();
    let mut row_of_step: HashMap<usize, u8> = HashMap::new();
    for (k, t) in tokens[..keep].iter().enumerate() {
        row_of_step.insert(t.step, wave.len() as u8);
        let arp = if chord && k + 1 == keep { 0x7F } else { t.arp };
        wave.push([t.wf, arp, 0]);
        if t.repeats > 0 && !(chord && k + 1 == keep) {
            wave.push([t.repeats as u8, 0x80, 0xFF]); // hold the row for more frames; no change of pitch or detune
        }
    }
    if let Some(&row) = loop_step.and_then(|l| row_of_step.get(&l)) {
        wave.push([0xFE, row, 0]);
    }

    let pulse_ptr = 16 + wave.len() * 3 + 1;
    let filter_ptr = pulse_ptr + pulse.len() * 3 + 1;
    let default_chord = if chord { ins.default_chord } else { 1 };
    let mut d = vec![
        0x0A, 0x0F, ins.sustain_release & 0xF0, ins.attack_decay, ins.sustain_release, 0, 0, 0, default_chord, octave as i8 as u8,
        pulse_ptr as u8, filter_ptr as u8, 0, 0, 0, 0x09,
    ];
    for r in &wave { d.extend_from_slice(r); }
    d.push(0xFF);
    for r in &pulse { d.extend_from_slice(r); }
    d.push(0xFF);
    for r in filter.iter() { d.extend_from_slice(r); } // the program's $FF is the size byte written after it
    d
}

/// Pulse width program: set the width, then (if it sweeps) add the speed for as many frames as it takes to reach
/// the end, and bounce between the limits forever.
fn pulse_program(ins: &Instrument, st: &mut Stats) -> Vec<[u8; 3]> {
    let (lo, hi) = (ins.pulse_min.min(4095) as i32, ins.pulse_max.min(4095) as i32);
    let pw0 = (ins.pulse_width.min(4095) as i32).clamp(lo.min(hi), hi.max(lo));
    let set = |w: i32| [0x80 | (w >> 8) as u8, w as u8, 0u8];
    let mut rows = vec![set(ins.pulse_width.min(4095) as i32)];
    let speed = ins.pulse_speed as i32;
    if speed == 0 || hi <= lo {
        return rows;
    }
    let mag = speed.abs();
    let leg = |frames: i32, delta: i32| -> Vec<[u8; 3]> {
        let mut v = Vec::new();
        let mut left = frames;
        while left > 0 {
            let n = left.min(127);
            v.push([n as u8, delta as i8 as u8, 0]);
            left -= n;
        }
        v
    };
    let end = if speed > 0 { hi } else { lo };
    let first = ((end - pw0).abs() + mag - 1) / mag;
    let full = (hi - lo + mag - 1) / mag;
    rows.extend(leg(first, speed));
    let start = rows.len();
    rows.extend(leg(full, -speed));
    rows.extend(leg(full, speed));
    rows.push([0xFE, start as u8, 0]);
    if rows.len() > 10 {
        // too slow a sweep for a program that small: sweep once towards the end
        st.pulse_simplified += 1;
        rows.truncate(1);
        rows.extend(leg(first, speed).into_iter().take(4));
    }
    rows
}
