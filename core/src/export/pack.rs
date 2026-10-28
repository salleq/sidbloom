//! Song -> memory image for `player/player.asm`. Layout constants must match the `.asm` header.
use crate::engine::note_freq;
use crate::model::*;

pub const LOAD_ADDR: u16 = 0x1000;
pub const INIT_ADDR: u16 = 0x1000;
pub const PLAY_ADDR: u16 = 0x1003;
pub const DATA_BASE: u16 = 0x1800;

const PLAYER: &[u8] = include_bytes!("../../player/player.bin");

pub const MAX_INSTRUMENTS: usize = 32;
pub const MAX_PATTERNS: usize = 64;
pub const MAX_ROWS: usize = 64;
pub const MAX_WAVE_STEPS: usize = 128;
pub const MAX_ORDER: usize = 255;

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
    if song.patterns.len() > MAX_PATTERNS { return Err(PackError::TooManyPatterns); }
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

    let base = DATA_BASE as usize;
    let mut img = PLAYER.to_vec();
    assert_eq!(img.len(), base - LOAD_ADDR as usize, "player.bin must fill up to the data area");
    img.resize(base - LOAD_ADDR as usize + 0x400, 0); // fixed tables; variable data follows
    let at = |off: usize| base - LOAD_ADDR as usize + off;

    let f = song.filter;
    img[at(0)] = song.instruments.len() as u8;
    img[at(1)] = song.speed.max(1);
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

    // Variable data region starts at DATA_BASE + 0x400; helper returns its absolute address.
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
        img[at(0x120 + i)] = inst.sustain_release;
        img[at(0x140 + i)] = inst.pulse_width as u8;
        img[at(0x160 + i)] = (inst.pulse_width >> 8) as u8 & 0x0F;
        img[at(0x180 + i)] = steps.len() as u8;
        img[at(0x1A0 + i)] = inst.wave_loop.map_or(0xFF, |l| l.min(steps.len() as u8 - 1));
        img[at(0x1C0 + i)] = addr as u8;
        img[at(0x1E0 + i)] = (addr >> 8) as u8;
        let (pmin, pmax) = (inst.pulse_min.min(0x0FFF), inst.pulse_max.min(0x0FFF));
        img[at(0x2C0 + i)] = inst.pulse_speed as u8;
        img[at(0x2E0 + i)] = pmin as u8;
        img[at(0x300 + i)] = (pmin >> 8) as u8;
        img[at(0x320 + i)] = pmax as u8;
        img[at(0x340 + i)] = (pmax >> 8) as u8;
    }

    for (ch, order) in song.orders.iter().enumerate() {
        let addr = push(&mut img, order)?;
        img[at(6 + ch)] = order.len() as u8;
        img[at(9 + ch)] = addr as u8;
        img[at(12 + ch)] = (addr >> 8) as u8;
    }

    for (i, pat) in song.patterns.iter().enumerate() {
        let bytes: Vec<u8> = pat.rows.iter().flat_map(|c| [c.note, c.inst, c.fx, c.fx_param]).collect();
        let addr = push(&mut img, &bytes)?;
        img[at(0x200 + i)] = pat.rows.len() as u8;
        img[at(0x240 + i)] = addr as u8;
        img[at(0x280 + i)] = (addr >> 8) as u8;
    }
    Ok(img)
}
