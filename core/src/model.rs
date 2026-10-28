//! Song data model. Kept small and fixed-size friendly so it maps 1:1 to the 6502 player data.
use serde::{Deserialize, Serialize};

pub const CHANNELS: usize = 3;
pub const NOTE_OFF: u8 = 0xFF;

/// Effect commands (high nibble semantics of the tracker effect column).
pub mod fx {
    pub const ARPEGGIO: u8 = 0x0; // param: x<<4 | y semitones
    pub const SLIDE_UP: u8 = 0x1; // param: freq units per tick
    pub const SLIDE_DOWN: u8 = 0x2;
    pub const VIBRATO: u8 = 0x4; // param: speed<<4 | depth
    pub const CHORD: u8 = 0x3; // param: chord number (1-based, 0 = none); only for instruments with a default chord
    pub const FILTER_CUTOFF: u8 = 0x8; // param: cutoff high byte (0..255)
    pub const FILTER_RES_ROUTE: u8 = 0x9; // param: resonance<<4 | voice routing bits
    pub const FILTER_MODE_VOL: u8 = 0xA; // param: mode bits (0x10 LP,0x20 BP,0x40 HP) | volume
    pub const FILTER_SWEEP: u8 = 0xB; // param: signed cutoff change per tick (until changed)
    pub const SET_SPEED: u8 = 0xF; // param: ticks per row
    pub const NONE: u8 = 0xFF;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// 0 = empty, 1..=95 = C-0..B-7, NOTE_OFF = gate off
    pub note: u8,
    /// 0 = keep current, else 1-based instrument number
    pub inst: u8,
    pub fx: u8,
    pub fx_param: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { note: 0, inst: 0, fx: fx::NONE, fx_param: 0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub rows: Vec<Cell>,
}

impl Pattern {
    pub fn new(len: usize) -> Self {
        Pattern { rows: vec![Cell::default(); len] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveStep {
    /// SID control register waveform bits (0x10 tri, 0x20 saw, 0x40 pulse, 0x80 noise, +ring 0x04, sync 0x02)
    pub wave: u8,
    /// semitone offset added to the note (signed)
    pub semis: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    pub name: String,
    pub attack_decay: u8,
    pub sustain_release: u8,
    pub pulse_width: u16, // 12 bit
    pub wave_table: Vec<WaveStep>,
    /// index the wave table loops back to after its last step; None = hold last step
    pub wave_loop: Option<u8>,
    /// Pulse width sweep: signed change per tick (0 = off), bouncing between min and max.
    #[serde(default)]
    pub pulse_speed: i8,
    #[serde(default)]
    pub pulse_min: u16,
    #[serde(default = "default_pulse_max")]
    pub pulse_max: u16,
    /// Chord (1-based index into `Song::chords`) the instrument plays on every note; 0 = not a chord instrument.
    #[serde(default)]
    pub default_chord: u8,
}

/// A chord: semitone offsets stepped through one per frame, looping (or holding the last one).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    pub notes: Vec<i8>,
    #[serde(default)]
    pub hold: bool,
}

fn default_pulse_max() -> u16 { 0x0FFF }

impl Instrument {
    pub fn simple(name: &str, wave: u8, ad: u8, sr: u8) -> Self {
        Instrument {
            name: name.into(),
            attack_decay: ad,
            sustain_release: sr,
            pulse_width: 0x800,
            wave_table: vec![WaveStep { wave, semis: 0 }],
            wave_loop: None,
            pulse_speed: 0,
            pulse_min: 0,
            pulse_max: 0x0FFF,
            default_chord: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterSettings {
    pub cutoff: u16, // 11 bit
    pub resonance: u8, // 4 bit
    /// bit0..2 route voice 1..3 through filter
    pub routing: u8,
    /// 0x10 LP, 0x20 BP, 0x40 HP
    pub mode: u8,
    pub volume: u8, // 0..15
}

impl Default for FilterSettings {
    fn default() -> Self {
        FilterSettings { cutoff: 0, resonance: 0, routing: 0, mode: 0, volume: 15 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub title: String,
    pub author: String,
    pub released: String,
    pub pal: bool,
    pub speed: u8, // ticks per row
    pub filter: FilterSettings,
    pub patterns: Vec<Pattern>,
    /// per channel: list of pattern indices
    pub orders: [Vec<u8>; CHANNELS],
    pub instruments: Vec<Instrument>,
    #[serde(default)]
    pub chords: Vec<Chord>,
}

impl Song {
    pub fn new() -> Self {
        Song {
            title: "Untitled".into(),
            author: String::new(),
            released: String::new(),
            pal: true,
            speed: 6,
            filter: FilterSettings::default(),
            patterns: vec![Pattern::new(32)],
            orders: [vec![0], vec![0], vec![0]],
            instruments: vec![Instrument::simple("Pulse", 0x40, 0x09, 0xA9)],
            chords: vec![],
        }
    }
    pub fn clock_hz(&self) -> f64 { if self.pal { 985_248.0 } else { 1_022_727.0 } }
    pub fn frame_hz(&self) -> f64 { if self.pal { 50.0 } else { 60.0 } }
}

impl Default for Song { fn default() -> Self { Self::new() } }
