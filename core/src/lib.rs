pub mod engine;
pub mod model;
pub use engine::Engine;
pub use model::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl_of(frame: &[(u8, u8)], ch: usize) -> u8 {
        frame.iter().find(|w| w.0 == (ch * 7 + 4) as u8).unwrap().1
    }

    fn song_with_note() -> Song {
        let mut s = Song::new();
        s.patterns[0].rows[0] = Cell { note: 58, inst: 1, ..Default::default() }; // A-4
        s
    }

    #[test]
    fn a4_frequency_pal() {
        // 440 Hz on PAL: 440*2^24/985248 = 7491.9 -> 7493 (well-known $1D45 ~ 7493)
        let f = engine::note_freq(58, 985_248.0);
        assert!((7490..=7495).contains(&f), "{f}");
    }

    #[test]
    fn first_row_sets_gate_and_frequency() {
        let log = engine::render_log(&song_with_note(), 1);
        let f = &log[0];
        assert_eq!(ctrl_of(f, 0), 0x41);
        let lo = f.iter().find(|w| w.0 == 0).unwrap().1 as u16;
        let hi = f.iter().find(|w| w.0 == 1).unwrap().1 as u16;
        assert_eq!(hi << 8 | lo, engine::note_freq(58, 985_248.0));
        // all channels share pattern 0 in the default song, so voice 2 plays too
        assert_eq!(ctrl_of(f, 1), 0x41);
    }

    #[test]
    fn gate_off_before_next_note() {
        let mut s = song_with_note();
        s.patterns[0].rows[1] = Cell { note: 60, inst: 1, ..Default::default() };
        let log = engine::render_log(&s, 12);
        assert_eq!(ctrl_of(&log[4], 0) & 1, 1);
        assert_eq!(ctrl_of(&log[5], 0) & 1, 0, "gate dropped on last tick of row");
        assert_eq!(ctrl_of(&log[6], 0) & 1, 1);
    }

    #[test]
    fn loops_after_order_end() {
        let mut e = Engine::new(song_with_note());
        for _ in 0..(6 * 32 - 1) { e.tick(); }
        assert!(!e.looped());
        e.tick(); e.tick(); e.tick(); e.tick(); e.tick(); e.tick();
        assert!(e.looped());
    }
}
