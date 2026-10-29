//! Song -> memory image for `player/player.asm`. Layout constants must match the `.asm` header.
use crate::engine::note_freq;
use crate::model::*;
use std::collections::HashMap;

pub const LOAD_ADDR: u16 = 0x1000;
pub const INIT_ADDR: u16 = 0x1000;
pub const PLAY_ADDR: u16 = 0x1003;
pub const DATA_BASE: u16 = 0x1800;

const PLAYER: &[u8] = include_bytes!("../../player/player.bin");

pub const MAX_INSTRUMENTS: usize = 64;
pub const MAX_PATTERNS: usize = 128;
pub const MAX_ROWS: usize = 64;
pub const MAX_WAVE_STEPS: usize = 128;
pub const MAX_ORDER: usize = 255;
pub const MAX_CHORDS: usize = 32;
pub const MAX_CHORD_LEN: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum PackError {
    TooManyInstruments,
    TooManyPatterns,
    BadPatternLength(usize),
    EmptyOrder(usize),
    OrderTooLong(usize),
    OrderRefersToMissingPattern { channel: usize, pattern: u8 },
    WaveTableTooLong(usize),
    OutOfMemory,
    TooManyChords,
    ChordTooLong(usize),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PackError {}

/// Returns the memory image starting at `LOAD_ADDR` (player + data).
pub fn pack_song(song: &Song) -> Result<Vec<u8>, PackError> {
    if song.instruments.len() > MAX_INSTRUMENTS { return Err(PackError::TooManyInstruments); }
    if song.chords.len() > MAX_CHORDS { return Err(PackError::TooManyChords); }
    if let Some(i) = song.chords.iter().position(|c| c.notes.len() > MAX_CHORD_LEN) { return Err(PackError::ChordTooLong(i)); }
    for (i, p) in song.patterns.iter().enumerate() {
        if p.rows.is_empty() || p.rows.len() > MAX_ROWS { return Err(PackError::BadPatternLength(i)); }
    }
    for (ch, o) in song.orders.iter().enumerate() {
        if o.is_empty() { return Err(PackError::EmptyOrder(ch)); }
        if o.len() > MAX_ORDER { return Err(PackError::OrderTooLong(ch)); }
        if let Some(&bad) = o.iter().find(|&&p| p as usize >= song.patterns.len()) {
            return Err(PackError::OrderRefersToMissingPattern { channel: ch, pattern: bad });
        }
    }

    // Identical patterns are stored once and unused ones not at all (linked patterns, for example, copy
    // patterns that voices shared): playback only depends on what each order position plays.
    let mut canon: Vec<&Pattern> = Vec::new();
    let mut slot: HashMap<&[Cell], u8> = HashMap::new();
    let mut orders: [Vec<u8>; CHANNELS] = Default::default();
    for (ch, order) in song.orders.iter().enumerate() {
        for &p in order {
            let rows = song.patterns[p as usize].rows.as_slice();
            let idx = *slot.entry(rows).or_insert_with(|| { canon.push(&song.patterns[p as usize]); (canon.len() - 1) as u8 });
            orders[ch].push(idx);
        }
    }
    if canon.len() > MAX_PATTERNS { return Err(PackError::TooManyPatterns); }

    let base = DATA_BASE as usize;
    let mut img = PLAYER.to_vec();
    assert_eq!(img.len(), base - LOAD_ADDR as usize, "player.bin must fill up to the data area");
    img.resize(base - LOAD_ADDR as usize + 0x660, 0); // fixed tables; variable data follows
    let at = |off: usize| base - LOAD_ADDR as usize + off;

    let f = song.filter;
    img[at(0)] = song.instruments.len() as u8;
    img[at(1)] = song.speed.max(1);
    img[at(15)] = song.chords.len() as u8;
    img[at(2)] = (f.cutoff & 7) as u8;
    img[at(3)] = (f.cutoff >> 3) as u8;
    img[at(4)] = (f.resonance << 4) | (f.routing & 7);
    img[at(5)] = (f.mode & 0x70) | (f.volume & 15);

    let clock = song.clock_hz();
    for n in 0..=96usize {
        let fr = note_freq(n as i32, clock);
        img[at(0x10 + n)] = fr as u8;
        img[at(0x80 + n)] = (fr >> 8) as u8;
    }

    // Variable data region starts at DATA_BASE + 0x660; helper returns its absolute address.
    let push = |img: &mut Vec<u8>, bytes: &[u8]| -> Result<u16, PackError> {
        let addr = LOAD_ADDR as usize + img.len();
        if addr + bytes.len() > 0xD000 { return Err(PackError::OutOfMemory); }
        img.extend_from_slice(bytes);
        Ok(addr as u16)
    };

    for (i, inst) in song.instruments.iter().enumerate() {
        let steps: Vec<WaveStep> = if inst.wave_table.is_empty() {
            vec![WaveStep { wave: 0x40, semis: 0 }]
        } else {
            inst.wave_table.clone()
        };
        if steps.len() > MAX_WAVE_STEPS { return Err(PackError::WaveTableTooLong(i)); }
        let bytes: Vec<u8> = steps.iter().flat_map(|s| [s.wave, s.semis as u8]).collect();
        let addr = push(&mut img, &bytes)?;
        img[at(0x100 + i)] = inst.attack_decay;
        img[at(0x140 + i)] = inst.sustain_release;
        img[at(0x180 + i)] = inst.pulse_width as u8;
        img[at(0x1C0 + i)] = (inst.pulse_width >> 8) as u8 & 0x0F;
        img[at(0x200 + i)] = steps.len() as u8;
        img[at(0x240 + i)] = inst.wave_loop.map_or(0xFF, |l| l.min(steps.len() as u8 - 1));
        img[at(0x280 + i)] = addr as u8;
        img[at(0x2C0 + i)] = (addr >> 8) as u8;
        let (pmin, pmax) = (inst.pulse_min.min(0x0FFF), inst.pulse_max.min(0x0FFF));
        img[at(0x620 + i)] = inst.default_chord;
        img[at(0x300 + i)] = inst.pulse_speed as u8;
        img[at(0x340 + i)] = pmin as u8;
        img[at(0x380 + i)] = (pmin >> 8) as u8;
        img[at(0x3C0 + i)] = pmax as u8;
        img[at(0x400 + i)] = (pmax >> 8) as u8;
    }

    for (ch, order) in orders.iter().enumerate() {
        let addr = push(&mut img, order)?;
        img[at(6 + ch)] = order.len() as u8;
        img[at(9 + ch)] = addr as u8;
        img[at(12 + ch)] = (addr >> 8) as u8;
    }

    for (i, pat) in canon.iter().enumerate() {
        let bytes: Vec<u8> = pat.rows.iter().flat_map(|c| [c.note, c.inst, c.fx, c.fx_param]).collect();
        let addr = push(&mut img, &bytes)?;
        img[at(0x440 + i)] = pat.rows.len() as u8;
        img[at(0x4C0 + i)] = addr as u8;
        img[at(0x540 + i)] = (addr >> 8) as u8;
    }
    for (i, chord) in song.chords.iter().enumerate() {
        let bytes: Vec<u8> = chord.notes.iter().map(|&n| n as u8).collect();
        let addr = push(&mut img, &bytes)?;
        img[at(0x5C0 + i)] = chord.notes.len() as u8 | if chord.hold { 0x80 } else { 0 };
        img[at(0x5E0 + i)] = addr as u8;
        img[at(0x600 + i)] = (addr >> 8) as u8;
    }
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::demo_song;

    fn note(n: u8) -> Cell { Cell { note: n, inst: 1, ..Default::default() } }

    /// A song whose voices share and repeat patterns, stored with duplicates and an unused pattern.
    fn messy() -> Song {
        let mut s = demo_song();
        let (a, b, c) = (s.patterns[0].clone(), s.patterns[1].clone(), s.patterns[2].clone());
        let mut other = a.clone();
        other.rows[2] = note(60);
        s.patterns = vec![a.clone(), b.clone(), c.clone(), other, a.clone(), b.clone(), c.clone(), Pattern::new(8)];
        s.orders = [vec![0, 3, 4], vec![1, 5, 1], vec![2, 6, 2]];
        s.linked = false;
        s
    }

    #[test]
    fn duplicate_and_unused_patterns_are_not_stored() {
        let m = messy();
        let mut tidy = m.clone();
        tidy.patterns = vec![m.patterns[0].clone(), m.patterns[1].clone(), m.patterns[2].clone(), m.patterns[3].clone()];
        tidy.orders = [vec![0, 3, 0], vec![1, 1, 1], vec![2, 2, 2]];
        assert_eq!(pack_song(&m).unwrap(), pack_song(&tidy).unwrap());
    }

    #[test]
    fn linked_layout_packs_to_the_same_image_as_the_unlinked_song() {
        let m = messy();
        // linked: one group of three patterns per distinct position (copies of what the voices played)
        let mut l = m.clone();
        l.patterns = vec![];
        l.orders = Default::default();
        for pos in 0..3 {
            for ch in 0..CHANNELS {
                l.patterns.push(m.patterns[m.orders[ch][pos] as usize].clone());
                l.orders[ch].push((3 * pos + ch) as u8);
            }
        }
        l.linked = true;
        assert_eq!(pack_song(&m).unwrap(), pack_song(&l).unwrap());
    }

    #[test]
    fn pattern_limit_counts_distinct_patterns() {
        let mut s = demo_song();
        let p = s.patterns[0].clone();
        s.patterns = vec![p; MAX_PATTERNS + 5];
        s.orders = [vec![0, 1, 130], vec![2, 3, 4], vec![5, 6, 7]];
        assert!(pack_song(&s).is_ok());
        let distinct: Vec<Pattern> = (0..MAX_PATTERNS as u8 + 1).map(|i| { let mut p = Pattern::new(4); p.rows[0] = note(i % 90 + 1); p.rows[1] = Cell { fx: 0, fx_param: i, ..Default::default() }; p }).collect();
        let mut s = demo_song();
        s.orders = [(0..=MAX_PATTERNS as u8).collect(), vec![0], vec![0]];
        s.patterns = distinct;
        assert_eq!(pack_song(&s), Err(PackError::TooManyPatterns));
    }
}
