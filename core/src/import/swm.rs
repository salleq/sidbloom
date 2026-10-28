//! SID-Wizard 1.x module (`.swm`) importer, written from SID-Wizard's published module
//! specification and user manual (SID-Wizard is public domain). Only subtune 1 of three-voice
//! modules is imported; features this tracker does not have are approximated and reported.
//!
//! File layout (all offsets relative to the `SWM1` id; the data is packed back to front):
//! 64-byte header, sequences, patterns, instruments, chord table, tempo programs, subtune tempos.
use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum SwmError {
    NotSwm,
    Truncated(&'static str),
    Inconsistent(String),
}

impl std::fmt::Display for SwmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwmError::NotSwm => write!(f, "not a SID-Wizard 1.x module (no SWM1 identifier)"),
            SwmError::Truncated(w) => write!(f, "module is truncated while reading {w}"),
            SwmError::Inconsistent(w) => write!(f, "module structure is inconsistent: {w}"),
        }
    }
}
impl std::error::Error for SwmError {}

pub struct SwmImport {
    pub song: Song,
    /// Things that could not be represented exactly.
    pub warnings: Vec<String>,
}

const HEADER: usize = 64;
const NAME_LEN: usize = 8;
const PACKED_MIN: u8 = 0x70;
const PACKED_MAX: u8 = 0x77;

struct RawInstrument {
    data: Vec<u8>, // header + tables, without the trailing size byte
    name: String,
}

struct Reader<'a> {
    d: &'a [u8],
    pos: isize, // index of the next byte to read going backwards
}

impl Reader<'_> {
    fn take(&mut self, what: &'static str) -> Result<u8, SwmError> {
        if self.pos < HEADER as isize - 1 {
            return Err(SwmError::Truncated(what));
        }
        let b = self.d[self.pos as usize];
        self.pos -= 1;
        Ok(b)
    }
    fn peek(&self) -> Option<u8> {
        (self.pos >= 0).then(|| self.d[self.pos as usize])
    }
    fn skip(&mut self, n: usize, what: &'static str) -> Result<(), SwmError> {
        self.pos -= n as isize;
        if self.pos < HEADER as isize - 1 {
            return Err(SwmError::Truncated(what));
        }
        Ok(())
    }
}

fn find_module(bytes: &[u8]) -> Option<&[u8]> {
    // bare, PRG-wrapped (2-byte load address) or PC64 .P00-wrapped (26-byte header + load address)
    [0usize, 2, 0x1C].into_iter().find(|&o| bytes.get(o..o + 4) == Some(b"SWM1")).map(|o| &bytes[o..])
}

pub fn import_swm(bytes: &[u8]) -> Result<SwmImport, SwmError> {
    let d = find_module(bytes).ok_or(SwmError::NotSwm)?;
    if d.len() < HEADER + 2 {
        return Err(SwmError::Truncated("header"));
    }
    let mut warnings = Vec::new();
    let seq_amount = d[12] as usize;
    let pat_amount = d[13] as usize;
    let inst_amount = d[14] as usize;
    let chord_len = d[15] as usize;
    let tempo_prog_len = d[16] as usize;
    let default_len = if d[11] == 0 { 32 } else { d[11] as usize };
    let subtunes = if seq_amount != 0 { (seq_amount - 1) / 3 + 1 } else { 1 };

    let mut r = Reader { d, pos: d.len() as isize - 1 };

    // subtune tempos (read from the very end)
    let mut tempos = vec![(0u8, 0u8); subtunes];
    for t in tempos.iter_mut().rev() {
        t.1 = r.take("subtune tempos")?;
        t.0 = r.take("subtune tempos")?;
    }
    r.skip(tempo_prog_len, "tempo programs")?;
    r.skip(chord_len, "chord table")?;

    // instruments
    let mut insts: Vec<RawInstrument> = Vec::with_capacity(inst_amount);
    for _ in 0..inst_amount {
        r.skip(NAME_LEN, "instrument name")?;
        let size_pos = r.pos;
        let size = d[size_pos as usize] as usize;
        let name_bytes = &d[size_pos as usize + 1..size_pos as usize + 1 + NAME_LEN];
        let name: String = name_bytes.iter().take_while(|&&b| b != 0).map(|&b| if (0x20..0x7F).contains(&b) { b as char } else { ' ' }).collect();
        let start = size_pos - size as isize;
        if start < HEADER as isize {
            return Err(SwmError::Truncated("instrument data"));
        }
        insts.push(RawInstrument { data: d[start as usize..size_pos as usize].to_vec(), name: name.trim().trim_end_matches('.').trim().to_string() });
        r.pos = start - 1;
    }
    insts.reverse(); // read last-to-first

    // patterns: [unpacked-size][length] at the end of each; NOP runs are packed
    struct RawPattern {
        rows: usize,
        bytes: Vec<u8>, // unpacked, terminated by 0xFF
    }
    let mut pats: Vec<RawPattern> = Vec::with_capacity(pat_amount);
    for _ in 0..pat_amount {
        let rows = r.take("pattern length")? as usize;
        let size = r.take("pattern size")? as usize;
        if size == 0 {
            return Err(SwmError::Inconsistent("pattern with size 0".into()));
        }
        let mut buf = vec![0u8; size];
        buf[size - 1] = 0xFF;
        let mut j = size as isize - 2;
        while j >= 0 {
            let b = r.take("pattern data")?;
            let prev = r.peek().unwrap_or(1);
            if (PACKED_MIN..=PACKED_MAX).contains(&b) && (prev == 0 || (PACKED_MIN..=PACKED_MAX).contains(&prev)) {
                // packed NOPs: the byte stands for (b - 0x70 + 2) empty rows (buffer is already zeroed)
                j -= (b - PACKED_MIN) as isize + 2;
            } else {
                buf[j as usize] = b;
                j -= 1;
            }
        }
        pats.push(RawPattern { rows, bytes: buf });
    }
    pats.reverse();

    // sequences (orderlists)
    let mut seqs: Vec<Vec<u8>> = vec![Vec::new(); seq_amount];
    for s in seqs.iter_mut().rev() {
        let size = r.take("sequence size")? as usize;
        let mut v = vec![0u8; size];
        for k in (0..size).rev() {
            v[k] = r.take("sequence data")?;
        }
        *s = v;
    }
    if r.pos != HEADER as isize - 1 {
        return Err(SwmError::Inconsistent(format!("{} unread bytes between header and data", r.pos - (HEADER as isize - 1))));
    }

    if subtunes > 1 {
        warnings.push(format!("module has {subtunes} subtunes; only the first one was imported"));
    }
    if d[4] > 1 {
        warnings.push(format!("module uses multispeed ({}x); it will play slower here", d[4]));
    }
    if chord_len > 0 {
        warnings.push("chord table is not imported (chords and chord arpeggios are lost)".into());
    }
    if tempo_prog_len > 0 {
        warnings.push("tempo programs are not imported".into());
    }

    // ---- song-level data
    let mut song = Song::new();
    song.title = d[24..HEADER].iter().take_while(|&&b| b != 0).map(|&b| if (0x20..0x7F).contains(&b) { b as char } else { ' ' }).collect::<String>().trim().chars().take(31).collect();
    let (t0, t1) = tempos[0];
    let speed = if t0 & 0x80 != 0 { (t0 & 0x7F) as u32 } else { (t0 as u32 + (t1 & 0x7F) as u32) / 2 };
    song.speed = speed.clamp(1, 31) as u8;
    if speed == 0 || speed > 31 {
        warnings.push(format!("tempo {speed} is out of range, using {}", song.speed));
    }
    if t0 & 0x80 == 0 && t0 != t1 & 0x7F {
        warnings.push("funk tempo was averaged to a single speed".into());
    }

    // ---- instruments
    song.instruments.clear();
    let mut ins_warn = InstWarnings::default();
    for raw in insts.iter().take(crate::export::pack::MAX_INSTRUMENTS) {
        song.instruments.push(convert_instrument(raw.name.clone(), &raw.data, &mut ins_warn));
    }
    if insts.len() > crate::export::pack::MAX_INSTRUMENTS {
        warnings.push(format!("{} instruments dropped (limit {})", insts.len() - crate::export::pack::MAX_INSTRUMENTS, crate::export::pack::MAX_INSTRUMENTS));
    }
    if song.instruments.is_empty() {
        song.instruments.push(Instrument::simple("Pulse", 0x40, 0x09, 0xA9));
    }
    ins_warn.report(&mut warnings);

    // ---- patterns
    let mut stats = FxStats::default();
    let n_inst = song.instruments.len();
    let parsed: Vec<Vec<Cell>> = pats.iter().map(|p| parse_pattern(&p.bytes, p.rows, n_inst, &mut stats)).collect();
    stats.report(&mut warnings);

    // ---- orderlists -> our pattern list (transposed copies and <=64-row chunks are separate patterns)
    song.patterns.clear();
    let mut cache: HashMap<(u8, i8), Vec<u8>> = HashMap::new();
    let mut orders: [Vec<u8>; 3] = [vec![], vec![], vec![]];
    let max_pat = crate::export::pack::MAX_PATTERNS;
    let mut overflow = false;
    let mut missing = false;
    let mut ignored_cmds = 0;
    for ch in 0..3 {
        let Some(seq) = seqs.get(ch) else { continue };
        let mut transpose = 0i8;
        let mut i = 0;
        while i < seq.len() {
            let v = seq[i];
            i += 1;
            match v {
                0 => {}
                1..=0x7F => {
                    let key = (v, transpose);
                    if !cache.contains_key(&key) {
                        let rows: Vec<Cell> = match parsed.get(v as usize - 1) {
                            Some(p) => p.iter().map(|c| transposed(c, transpose)).collect(),
                            None => {
                                missing = true;
                                vec![emptycell(); default_len]
                            }
                        };
                        let rows = if rows.is_empty() { vec![emptycell(); default_len] } else { rows };
                        let mut ids = Vec::new();
                        for chunk in rows.chunks(crate::export::pack::MAX_ROWS) {
                            if song.patterns.len() >= max_pat {
                                overflow = true;
                                break;
                            }
                            ids.push(song.patterns.len() as u8);
                            song.patterns.push(Pattern { rows: chunk.to_vec() });
                        }
                        cache.insert(key, ids);
                    }
                    orders[ch].extend_from_slice(&cache[&key]);
                }
                0x80..=0x9F => transpose = (v as i16 - 0x90) as i8,
                0xA0..=0xFD => ignored_cmds += 1, // main volume / track tempo
                0xFE => break,
                0xFF => {
                    let target = seq.get(i).copied().unwrap_or(0);
                    if target != 0 {
                        warnings.push(format!("voice {} loops back to position {target:02X}; it loops to the start here", ch + 1));
                    }
                    break;
                }
            }
        }
    }
    if overflow {
        warnings.push(format!("pattern limit ({max_pat}) reached after expanding transposes and long patterns; the song is cut short"));
    }
    if missing {
        warnings.push("orderlist refers to patterns that do not exist; empty patterns were used".into());
    }
    if ignored_cmds > 0 {
        warnings.push(format!("{ignored_cmds} orderlist volume/tempo commands were ignored"));
    }
    if song.patterns.is_empty() {
        song.patterns.push(Pattern::new(default_len.min(64)));
    }
    for o in orders.iter_mut() {
        if o.is_empty() {
            // an unused voice: a silent pattern
            let id = song.patterns.len() as u8;
            song.patterns.push(Pattern::new(song.patterns[0].rows.len()));
            o.push(id);
        }
    }
    song.orders = orders;
    Ok(SwmImport { song, warnings })
}

fn emptycell() -> Cell {
    Cell::default()
}

fn transposed(c: &Cell, t: i8) -> Cell {
    let mut c = *c;
    if t != 0 && (1..=95).contains(&c.note) {
        c.note = (c.note as i32 + t as i32).clamp(1, 95) as u8;
    }
    c
}

#[derive(Default)]
struct FxStats {
    note_fx: usize,
    small_fx: usize,
    big_fx: HashMap<u8, usize>,
    bad_inst: usize,
    row_mismatch: usize,
}

impl FxStats {
    fn report(&self, w: &mut Vec<String>) {
        if self.note_fx > 0 {
            w.push(format!("{} note-column effects (vibrato/portamento/sync/ring/gate-on) were not imported", self.note_fx));
        }
        if self.small_fx > 0 {
            w.push(format!("{} small effects (attack/decay/sustain/release/volume/vibrato settings/…) were not imported", self.small_fx));
        }
        let mut big: Vec<_> = self.big_fx.iter().collect();
        big.sort();
        for (fx, n) in big {
            w.push(format!("{n} uses of effect {fx:02X} are not supported and were dropped"));
        }
        if self.bad_inst > 0 {
            w.push(format!("{} rows selected an instrument that was not imported", self.bad_inst));
        }
        if self.row_mismatch > 0 {
            w.push(format!("{} patterns had a different row count than their data; they were padded or cut", self.row_mismatch));
        }
    }
}

fn parse_pattern(b: &[u8], rows: usize, n_inst: usize, st: &mut FxStats) -> Vec<Cell> {
    let mut out: Vec<Cell> = Vec::with_capacity(rows);
    let mut i = 0;
    while i < b.len() && b[i] != 0xFF && out.len() < 256 {
        let nb = b[i];
        i += 1;
        let mut cell = Cell::default();
        let note = nb & 0x7F;
        match note {
            0 => {}
            1..=0x5F => cell.note = note,
            0x7E => cell.note = NOTE_OFF,
            _ => st.note_fx += 1,
        }
        if nb & 0x80 != 0 {
            let ib = b.get(i).copied().unwrap_or(0);
            i += 1;
            let ins = ib & 0x7F;
            match ins {
                1..=0x3E => {
                    if (ins as usize) <= n_inst {
                        cell.inst = ins;
                    } else {
                        st.bad_inst += 1;
                    }
                }
                0x3F | 0 => {}
                _ => st.small_fx += 1,
            }
            if ib & 0x80 != 0 {
                let fx = b.get(i).copied().unwrap_or(0);
                i += 1;
                if fx < 0x20 {
                    let val = b.get(i).copied().unwrap_or(0);
                    i += 1;
                    match map_big_fx(fx, val) {
                        Some((f, p)) => {
                            cell.fx = f;
                            cell.fx_param = p;
                        }
                        None => *st.big_fx.entry(fx).or_insert(0) += 1,
                    }
                } else {
                    st.small_fx += 1;
                }
            }
        }
        out.push(cell);
    }
    if out.len() != rows {
        st.row_mismatch += 1;
        out.resize(rows, Cell::default());
    }
    out
}

fn map_big_fx(fx: u8, val: u8) -> Option<(u8, u8)> {
    match fx {
        0x01 => Some((crate::model::fx::SLIDE_UP, val)),
        0x02 => Some((crate::model::fx::SLIDE_DOWN, val)),
        0x08 => Some((crate::model::fx::VIBRATO, val)),
        0x0F => Some((crate::model::fx::FILTER_CUTOFF, val)),
        0x10 | 0x13 if val & 0x3F != 0 => Some((crate::model::fx::SET_SPEED, val & 0x3F)),
        _ => None,
    }
}

#[derive(Default)]
struct InstWarnings {
    absolute_pitch: usize,
    chord_jump: usize,
    filter_tables: usize,
    vibrato: usize,
    multispeed: usize,
    complex_pulse: usize,
    long_tables: usize,
}

impl InstWarnings {
    fn report(&self, w: &mut Vec<String>) {
        let items = [
            (self.absolute_pitch, "instruments use absolute pitches in the wave-arp table (treated as no pitch shift)"),
            (self.chord_jump, "instruments jump to chords in the wave-arp table (treated as no pitch shift)"),
            (self.filter_tables, "instruments have filter programs (not imported; use the filter effects instead)"),
            (self.vibrato, "instruments have built-in vibrato (not imported)"),
            (self.multispeed, "instruments use a wave-arp speed other than 1x (played at 1x)"),
            (self.complex_pulse, "instruments have pulse programs that were simplified to one sweep"),
            (self.long_tables, "instruments have wave tables that were cut to 128 steps"),
        ];
        for (n, text) in items {
            if n > 0 {
                w.push(format!("{n} {text}"));
            }
        }
    }
}

fn convert_instrument(name: String, d: &[u8], w: &mut InstWarnings) -> Instrument {
    let mut ins = Instrument::simple(&name, 0x40, d.get(3).copied().unwrap_or(0x09), d.get(4).copied().unwrap_or(0xA9));
    if d.len() < 0x11 {
        return ins;
    }
    if d[5] != 0 {
        w.vibrato += 1;
    }
    if d[7] & 0x3F != 0 {
        w.multispeed += 1;
    }
    let octave = d[9] as i8 as i16;
    let pulse_ptr = (d[0x0A] as usize).clamp(0x10, d.len());
    let filter_ptr = (d[0x0B] as usize).clamp(pulse_ptr, d.len());
    if filter_ptr < d.len() && d[filter_ptr] != 0xFF {
        w.filter_tables += 1;
    }

    // ---- wave / arp / detune table (3-byte rows, 0xFF ends)
    let mut steps: Vec<WaveStep> = Vec::new();
    let first_frame = d[0x0F];
    if first_frame & 0xF0 != 0 {
        steps.push(WaveStep { wave: first_frame & 0xFE, semis: shift(0, octave) });
    }
    let mut row_start: Vec<usize> = Vec::new(); // SW row -> index of its first step
    let mut loop_row: Option<u8> = None;
    let (mut last_wave, mut last_semis) = (0x40u8, 0i16);
    let mut absolute = false;
    let mut chord = false;
    let mut i = 0x10;
    while i + 2 < pulse_ptr.max(i) && i < d.len() && d[i] != 0xFF {
        let (wf, arp) = (d[i], d[i + 1]);
        row_start.push(steps.len());
        if wf == 0xFE {
            let target = arp as usize;
            if target <= 0x40 && target < row_start.len() {
                loop_row = Some(row_start[target].min(steps.len().saturating_sub(1)) as u8);
            }
            break;
        }
        if wf <= 0x0F {
            // repeat the previous row `wf` more times
            for _ in 0..wf {
                steps.push(WaveStep { wave: last_wave, semis: shift(last_semis, octave) });
            }
        } else {
            last_wave = wf & 0xFE;
            match arp {
                0x00 => last_semis = 0,
                0x01..=0x5F => last_semis = arp as i16,
                0x7F => chord = true,
                0x80 => {}
                0x81..=0xDF => absolute = true,
                0xE0..=0xFF => last_semis = arp as i8 as i16,
                _ => {}
            }
            steps.push(WaveStep { wave: last_wave, semis: shift(last_semis, octave) });
        }
        i += 3;
    }
    if absolute {
        w.absolute_pitch += 1;
    }
    if chord {
        w.chord_jump += 1;
    }
    if steps.is_empty() {
        steps.push(WaveStep { wave: last_wave, semis: shift(0, octave) });
    }
    if steps.len() > crate::export::pack::MAX_WAVE_STEPS {
        steps.truncate(crate::export::pack::MAX_WAVE_STEPS);
        w.long_tables += 1;
    }
    let prefix = (first_frame & 0xF0 != 0) as usize;
    ins.wave_loop = loop_row.map(|l| (l as usize + prefix).min(steps.len() - 1) as u8).filter(|&l| (l as usize) < steps.len());
    ins.wave_table = steps;

    // ---- pulse program: "set" rows ($8x..$Fx xx) and "add" rows ($00..$7F cycles, xx delta)
    let mut start: Option<u16> = None;
    let mut sweep: Option<(u8, i8)> = None;
    let mut rows = 0;
    let mut i = pulse_ptr;
    while i + 1 < d.len() && d[i] != 0xFF && i < filter_ptr {
        let (b0, b1) = (d[i], d[i + 1]);
        rows += 1;
        if b0 == 0xFE {
            // jump: keep what we have
        } else if b0 >= 0x80 {
            if start.is_none() && sweep.is_none() {
                start = Some((((b0 & 0x0F) as u16) << 8) | b1 as u16);
            }
        } else if sweep.is_none() {
            sweep = Some((b0, b1 as i8));
        }
        i += 3;
    }
    if rows > 2 || (rows == 2 && (start.is_none() || sweep.is_none())) {
        w.complex_pulse += 1;
    }
    let pw0 = start.unwrap_or(0x800);
    ins.pulse_width = pw0;
    if let Some((cycles, delta)) = sweep.filter(|&(c, dl)| c > 0 && dl != 0) {
        let end = (pw0 as i32 + cycles as i32 * delta as i32).clamp(0, 4095) as u16;
        ins.pulse_speed = delta;
        ins.pulse_min = pw0.min(end);
        ins.pulse_max = pw0.max(end);
    }
    ins
}

fn shift(semis: i16, octave: i16) -> i8 {
    (semis + octave).clamp(-127, 127) as i8
}
