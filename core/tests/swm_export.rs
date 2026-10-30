//! SID-Wizard module export: songs go through `export_swm` and come back through `import_swm` playing the same, plus
//! (optionally) every module found in the directory named by `SIDBLOOM_SWM_DIR`.
use sidcore::engine::render_log;
use sidcore::export::swm::{export_swm, SwmExportError};
use sidcore::import::swm::import_swm;
use sidcore::*;

fn cell(note: u8, inst: u8) -> Cell {
    Cell { note, inst, ..Default::default() }
}

fn fxcell(note: u8, inst: u8, fx: u8, fx_param: u8) -> Cell {
    Cell { note, inst, fx, fx_param }
}

fn sample_song() -> Song {
    let mut s = Song::new();
    s.title = "Round Trip".into();
    s.author = "Someone".into();
    s.speed = 5;
    s.chords = vec![Chord { notes: vec![0, 4, 7], hold: false }, Chord { notes: vec![0, 3, 7, 12], hold: true }];
    let mut lead = Instrument::simple("Lead", 0x40, 0x09, 0xA9);
    lead.pulse_width = 0x400;
    lead.pulse_speed = 16;
    lead.pulse_min = 0x400;
    lead.pulse_max = 0xC00;
    lead.wave_table = vec![
        WaveStep { wave: 0x80, semis: 0 },
        WaveStep { wave: 0x40, semis: 0 },
        WaveStep { wave: 0x40, semis: 0 },
        WaveStep { wave: 0x40, semis: 0 },
        WaveStep { wave: 0x40, semis: 12 },
        WaveStep { wave: 0x20, semis: -5 },
    ];
    lead.wave_loop = Some(1);
    let mut arp = Instrument::simple("Arp", 0x20, 0x08, 0x88);
    arp.default_chord = 2;
    let tri = Instrument::simple("Tri", 0x10, 0x00, 0xF0);
    s.instruments = vec![lead, arp, tri];

    let mut a = Pattern::new(16);
    a.rows[0] = cell(49, 1);
    a.rows[2] = cell(52, 0);
    a.rows[3] = cell(NOTE_OFF, 0);
    a.rows[4] = fxcell(56, 1, fx::SLIDE_UP, 0x30);
    a.rows[8] = fxcell(0, 0, fx::VIBRATO, 0x42);
    a.rows[12] = fxcell(61, 3, fx::FILTER_CUTOFF, 0x55);
    a.rows[15] = fxcell(0, 0, fx::SET_SPEED, 6);
    let mut b = Pattern::new(16);
    b.rows[0] = fxcell(37, 2, fx::CHORD, 1);
    b.rows[1] = fxcell(0, 0, fx::CHORD, 2);
    b.rows[6] = cell(40, 2);
    b.rows[10] = fxcell(44, 3, fx::SLIDE_DOWN, 0x10);
    s.patterns = vec![a, b, Pattern::new(8)];
    s.orders = [vec![0, 1, 0], vec![1, 1], vec![2, 0]];
    s
}

fn same_playback(a: &Song, b: &Song, frames: usize, what: &str) {
    let (la, lb) = (render_log(a, frames), render_log(b, frames));
    for (i, (x, y)) in la.iter().zip(&lb).enumerate() {
        assert_eq!(x, y, "{what}: register writes differ in frame {i}");
    }
}

#[test]
fn exported_module_imports_and_plays_the_same() {
    let song = sample_song();
    let out = export_swm(&song).unwrap();
    assert_eq!(&out.bytes[..4], b"SWM1");
    let imp = import_swm(&out.to_prg()).unwrap();
    let s = &imp.song;
    assert_eq!((s.title.as_str(), s.author.as_str(), s.speed), ("ROUND TRIP", "SOMEONE", 5));
    assert_eq!(s.chords, song.chords);
    assert_eq!(s.instruments.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["LEAD", "ARP", "TRI"]);
    assert_eq!(s.instruments[0].wave_table, song.instruments[0].wave_table);
    assert_eq!(s.instruments[0].wave_loop, Some(1));
    let l = &s.instruments[0];
    assert_eq!((l.pulse_width, l.pulse_speed, l.pulse_min, l.pulse_max), (0x400, 16, 0x400, 0xC00));
    assert_eq!(s.instruments[1].default_chord, 2);
    assert_eq!(s.instruments[2].default_chord, 0);
    assert!(imp.warnings.iter().all(|w| !w.contains("not supported")), "{:?}", imp.warnings);
    // the first pattern's rows survive (plus the stop rows of its slide and vibrato)
    let first = &s.patterns[s.orders[0][0] as usize].rows;
    assert_eq!((first[0].note, first[0].inst), (49, 1));
    assert_eq!(first[3].note, NOTE_OFF);
    assert_eq!((first[4].fx, first[4].fx_param), (fx::SLIDE_UP, 0x30));
    assert_eq!((first[5].fx, first[5].fx_param), (fx::SLIDE_UP, 0), "slide is stopped explicitly");
    assert_eq!((first[8].fx, first[8].fx_param), (fx::VIBRATO, 0x42));
    assert_eq!((first[12].fx, first[12].fx_param), (fx::FILTER_CUTOFF, 0x55));
    assert_eq!((first[15].fx, first[15].fx_param), (fx::SET_SPEED, 6));
    same_playback(&song, s, 6 * 16 * 6, "sample song");
}

#[test]
fn identical_patterns_are_stored_once_and_long_ones_split() {
    let mut s = Song::new();
    let mut busy = Pattern::new(64);
    for (i, r) in busy.rows.iter_mut().enumerate() {
        *r = fxcell(1 + (i % 90) as u8, 1, fx::VIBRATO, 0x11 + i as u8);
    }
    s.patterns = vec![busy.clone(), busy, Pattern::new(32)];
    s.orders = [vec![0, 1], vec![1], vec![2]];
    let out = export_swm(&s).unwrap();
    let imp = import_swm(&out.bytes).unwrap();
    // 64 rows of 4 bytes do not fit a SID-Wizard pattern: two chunks, and both copies share them
    assert_eq!(out.bytes[13], 3, "two chunks of the busy pattern and one empty pattern");
    assert_eq!(imp.song.orders[0].len(), 4);
    assert_eq!(imp.song.orders[1].len(), 2);
    same_playback(&s, &imp.song, 6 * 64 * 2, "busy pattern");
}

#[test]
fn empty_runs_are_packed_the_way_the_player_unpacks_them() {
    for rows in [1usize, 2, 3, 9, 10, 11, 12, 20, 33, 64] {
        let mut s = Song::new();
        let mut p = Pattern::new(rows);
        p.rows[0] = cell(30, 1);
        p.rows[rows - 1] = cell(32, 1);
        s.patterns = vec![p];
        s.orders = [vec![0], vec![0], vec![0]];
        let imp = import_swm(&export_swm(&s).unwrap().bytes).unwrap();
        assert_eq!(imp.song.patterns[0].rows.len(), rows);
        assert_eq!(imp.song.patterns[0].rows, s.patterns[0].rows, "{rows} rows");
    }
}

#[test]
fn song_filter_becomes_an_instrument_filter_program() {
    let mut s = sample_song();
    s.filter = FilterSettings { cutoff: 0x400, resonance: 7, routing: 0b001, mode: 0x10, volume: 12 };
    let out = export_swm(&s).unwrap();
    let imp = import_swm(&out.bytes).unwrap();
    assert!(imp.warnings.iter().any(|w| w.contains("filter programs")), "voice 1's instruments carry a filter program: {:?}", imp.warnings);
    // main volume command at the start of the first sequence ($AC)
    assert!(imp.warnings.iter().any(|w| w.contains("orderlist volume/tempo")), "{:?}", imp.warnings);
}

#[test]
fn what_cannot_be_represented_is_reported() {
    let mut s = sample_song();
    s.patterns[0].rows[1] = fxcell(0, 0, fx::ARPEGGIO, 0x37);
    s.patterns[0].rows[9] = fxcell(0, 0, fx::FILTER_SWEEP, 2);
    s.released = "2026".into();
    let out = export_swm(&s).unwrap();
    let text = out.warnings.join("\n");
    assert!(text.contains("arpeggio"), "{text}");
    assert!(text.contains("filter sweep"), "{text}");
    assert!(text.contains("Released"), "{text}");
}

#[test]
fn too_big_songs_are_refused_not_cut() {
    let mut s = Song::new();
    s.orders = [vec![0; 127], vec![0], vec![0]];
    assert!(matches!(export_swm(&s), Err(SwmExportError::SequenceTooLong { voice: 1, .. })));
    let mut s = Song::new();
    s.instruments = vec![Instrument::simple("x", 0x40, 0, 0xF0); 63];
    assert!(matches!(export_swm(&s), Err(SwmExportError::TooManyInstruments(63))));
    let mut s = Song::new();
    s.patterns = (0..130).map(|i| { let mut p = Pattern::new(4); p.rows[0] = cell(1 + (i % 90) as u8, 1); p.rows[1] = fxcell(0, 0, fx::FILTER_CUTOFF, i as u8); p }).collect();
    s.orders = [(0..126).collect(), (126..130).collect(), vec![0]];
    assert!(matches!(export_swm(&s), Err(SwmExportError::TooManyPatterns(128))));
}

#[test]
fn real_modules_survive_export_and_import() {
    let Ok(dir) = std::env::var("SIDBLOOM_SWM_DIR") else {
        eprintln!("SIDBLOOM_SWM_DIR not set, skipping");
        return;
    };
    let (mut ok, mut refused) = (0, 0);
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            stack.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_lowercase();
        if !(name.ends_with(".swm.prg") || name.ends_with(".swm.p00") || name.ends_with(".swm")) {
            continue;
        }
        let Ok(first) = import_swm(&std::fs::read(&p).unwrap()) else { continue };
        let out = match export_swm(&first.song) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("refused {name}: {e}");
                refused += 1;
                continue;
            }
        };
        let again = import_swm(&out.to_prg()).unwrap_or_else(|e| panic!("{name}: exported module does not import: {e}"));
        same_playback(&first.song, &again.song, 50 * 20, &name);
        eprintln!("ok {name}: {} bytes, {} warnings", out.bytes.len(), out.warnings.len());
        ok += 1;
    }
    assert!(ok > 0, "no modules found");
    eprintln!("{ok} modules round-tripped, {refused} refused for size");
}
