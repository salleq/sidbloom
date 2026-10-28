pub mod audio;
pub mod engine;
pub mod export;
pub mod model;
pub mod project;
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

    fn reg(frame: &[(u8, u8)], r: u8) -> u8 {
        frame.iter().find(|w| w.0 == r).unwrap().1
    }
    fn pw(frame: &[(u8, u8)]) -> u16 {
        reg(frame, 2) as u16 | (reg(frame, 3) as u16) << 8
    }

    #[test]
    fn pulse_sweep_bounces_between_limits() {
        let mut s = song_with_note();
        let i = &mut s.instruments[0];
        i.pulse_width = 0x200;
        i.pulse_speed = 0x60;
        i.pulse_min = 0x100;
        i.pulse_max = 0x300;
        let log = engine::render_log(&s, 12);
        let widths: Vec<u16> = log.iter().map(|f| pw(f)).collect();
        assert_eq!(&widths[..5], &[0x200, 0x260, 0x2C0, 0x300, 0x2A0]);
        assert!(widths.iter().all(|w| (0x100..=0x300).contains(w)), "{widths:x?}");
        assert!(widths.iter().any(|&w| w == 0x300) && widths.iter().any(|&w| w < 0x2A0));
    }

    #[test]
    fn filter_effects_change_registers() {
        let mut s = song_with_note();
        s.patterns[0].rows[1] = Cell { fx: fx::FILTER_CUTOFF, fx_param: 0x40, ..Default::default() };
        s.patterns[0].rows[2] = Cell { fx: fx::FILTER_RES_ROUTE, fx_param: 0xF1, ..Default::default() };
        s.patterns[0].rows[3] = Cell { fx: fx::FILTER_MODE_VOL, fx_param: 0x1A, ..Default::default() };
        s.patterns[0].rows[4] = Cell { fx: fx::FILTER_SWEEP, fx_param: 0xFE, ..Default::default() }; // -2 per tick
        let log = engine::render_log(&s, 6 * 6);
        assert_eq!(reg(&log[0], 22), 0);
        assert_eq!(reg(&log[6], 22), 0x40);
        assert_eq!(reg(&log[12], 23), 0xF1);
        assert_eq!(reg(&log[18], 24), 0x1A);
        let before = reg(&log[24], 22);
        assert_eq!(reg(&log[25], 22), before.saturating_sub(2));
        assert_eq!(reg(&log[35], 22), before.saturating_sub(22));
    }
}
