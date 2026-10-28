//! Project files (`.sidb`): the `Song` as pretty-printed JSON.
use crate::model::Song;
use std::{fs, io, path::Path};

pub const EXTENSION: &str = "sidb";

pub fn to_json(song: &Song) -> String {
    serde_json::to_string_pretty(song).expect("Song serializes")
}

pub fn from_json(s: &str) -> Result<Song, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn save(song: &Song, path: &Path) -> io::Result<()> {
    fs::write(path, to_json(song))
}

pub fn load(path: &Path) -> io::Result<Song> {
    from_json(&fs::read_to_string(path)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// A short example tune used by `sidbloom demo` and the tests.
pub fn demo_song() -> Song {
    use crate::model::*;
    let mut s = Song::new();
    s.title = "SIDbloom demo".into();
    s.author = "SIDbloom".into();
    s.speed = 6;
    let mut lead = Instrument::simple("Lead", 0x41, 0x08, 0xA8);
    lead.pulse_speed = 12;
    lead.pulse_min = 0x300;
    lead.pulse_max = 0xD00;
    let mut bass = Instrument::simple("Bass", 0x21, 0x05, 0x85);
    bass.pulse_width = 0x400;
    let mut hat = Instrument::simple("Hat", 0x81, 0x00, 0x09);
    hat.wave_table.push(WaveStep { wave: 0x80, semis: 0 });
    hat.wave_loop = Some(1);
    s.instruments = vec![lead, bass, hat];
    let c = |note: u8, inst: u8, fx: u8, p: u8| Cell { note, inst, fx, fx_param: p };
    let e = Cell::default();
    let mut lead = Pattern::new(32);
    // A minor arpeggio line, with vibrato on the long notes
    let melody = [(58u8, 0u8), (0, 0), (61, 0), (0, 0), (65, 0), (0, 0), (64, 0), (0, 0)];
    for (i, (n, _)) in melody.iter().enumerate() {
        lead.rows[i * 4] = if *n == 0 { e } else { c(*n, 1, fx::VIBRATO, 0x23) };
    }
    lead.rows[30] = c(NOTE_OFF, 0, fx::NONE, 0);
    let mut bass = Pattern::new(32);
    for (i, n) in [34u8, 34, 37, 32].iter().enumerate() {
        bass.rows[i * 8] = c(*n, 2, fx::NONE, 0);
        bass.rows[i * 8 + 4] = c(*n + 12, 2, fx::NONE, 0);
    }
    let mut hat = Pattern::new(32);
    for i in (0..32).step_by(2) {
        hat.rows[i] = c(70, 3, fx::NONE, 0);
    }
    s.patterns = vec![lead, bass, hat];
    s.orders = [vec![0, 0], vec![1, 1], vec![2, 2]];
    s.filter = FilterSettings { cutoff: 0, resonance: 0, routing: 0, mode: 0, volume: 15 };
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let s = demo_song();
        assert_eq!(from_json(&to_json(&s)).unwrap(), s);
    }

    #[test]
    fn file_roundtrip() {
        let p = std::env::temp_dir().join("sidbloom_roundtrip_test.sidb");
        let s = demo_song();
        save(&s, &p).unwrap();
        assert_eq!(load(&p).unwrap(), s);
        std::fs::remove_file(p).ok();
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(from_json("not json").is_err());
    }
}
