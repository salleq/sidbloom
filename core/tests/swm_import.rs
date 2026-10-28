//! SID-Wizard module import: a hand-built module with known content, plus (optionally) every
//! module found in the directory named by `SIDBLOOM_SWM_DIR`.
mod common;
use common::assert_same;
use sidcore::export::pack_song;
use sidcore::import::swm::{import_swm, SwmError};
use sidcore::*;

/// Builds a minimal SWM1 module: 3 voices playing one 9-row pattern, 1 instrument.
fn build_module() -> Vec<u8> {
    let mut m = vec![0u8; 64];
    m[..4].copy_from_slice(b"SWM1");
    m[4] = 1; // frame speed
    m[11] = 32; // default pattern length
    m[12] = 3; // sequences
    m[13] = 1; // patterns
    m[14] = 1; // instruments
    m[15] = 9; // chord table bytes
    m[24..24 + 9].copy_from_slice(b"TEST TUNE");
    // sequences: [pattern 1, end] + size byte
    for _ in 0..3 {
        m.extend_from_slice(&[1, 0xFE, 2]);
    }
    // sequences: [pattern 1, end] + size byte (above); chord table sits after instruments (below)
    // pattern: row 0 = C-4 + instrument 1 + effect 0F (cutoff) 40, then 7 empty rows (0 + packed run of 6), row 8 = D-4
    m.extend_from_slice(&[0xB1, 0x81, 0x0F, 0x40, 0x00, 0x74, 0x32]);
    m.push(13); // unpacked size incl. terminator
    m.push(9); // rows
    // instrument: 16-byte header, wave table (2 rows), pulse program (set + add), empty filter program
    let mut inst = vec![0u8; 0x10];
    inst[3] = 0x09;
    inst[4] = 0xA9;
    inst[8] = 2; // default chord
    inst.extend_from_slice(&[0x41, 0x00, 0x00, 0x41, 0x7F, 0x00, 0xFF]); // 2nd row jumps to the chord
    inst[0x0A] = inst.len() as u8;
    inst.extend_from_slice(&[0x88, 0x00, 0x00, 0x10, 0x20, 0x00, 0xFF]);
    inst[0x0B] = inst.len() as u8;
    inst.push(0xFF);
    m.extend_from_slice(&inst);
    m.push(inst.len() as u8); // size byte replaces the final terminator in the packed file
    m.extend_from_slice(b"TEST    ");
    m.extend_from_slice(&[0x00, 0x03, 0x07, 0x7E, 0x00, 0x04, 0x07, 0x0C, 0x7F]); // chord 1 (hold), chord 2 (loop)
    m.extend_from_slice(&[0x86, 0x86]); // subtune tempo: single tempo 6
    m
}

#[test]
fn imports_hand_built_module() {
    let imp = import_swm(&build_module()).unwrap();
    let s = &imp.song;
    assert_eq!(s.title, "TEST TUNE");
    assert_eq!(s.speed, 6);
    assert_eq!(s.patterns.len(), 1);
    assert_eq!(s.orders, [vec![0], vec![0], vec![0]]);
    let rows = &s.patterns[0].rows;
    assert_eq!(rows.len(), 9);
    assert_eq!((rows[0].note, rows[0].inst, rows[0].fx, rows[0].fx_param), (49, 1, fx::FILTER_CUTOFF, 0x40));
    assert!(rows[1..8].iter().all(|c| c.note == 0 && c.fx == fx::NONE), "packed NOP run expands to empty rows");
    assert_eq!(rows[8].note, 0x32);
    let i = &s.instruments[0];
    assert_eq!(i.name, "TEST");
    assert_eq!((i.attack_decay, i.sustain_release), (0x09, 0xA9));
    assert_eq!(i.wave_table, vec![WaveStep { wave: 0x40, semis: 0 }, WaveStep { wave: 0x40, semis: 0 }]);
    assert_eq!(s.chords, vec![Chord { notes: vec![0, 3, 7], hold: true }, Chord { notes: vec![0, 4, 7, 12], hold: false }]);
    assert_eq!(i.default_chord, 2);
    assert_eq!(i.wave_loop, None);
    assert_eq!((i.pulse_width, i.pulse_speed, i.pulse_min, i.pulse_max), (0x800, 0x20, 0x800, 0xA00));
    // the result is a valid, exportable song whose player output equals the engine's
    pack_song(s).unwrap();
    assert_same(s, 200, "hand-built swm");
}

#[test]
fn accepts_prg_and_p00_wrappers() {
    let m = build_module();
    let mut prg = vec![0x00, 0x20];
    prg.extend_from_slice(&m);
    assert!(import_swm(&prg).is_ok());
    let mut p00 = b"C64File\0".to_vec();
    p00.extend_from_slice(&[b'T'; 16]);
    p00.extend_from_slice(&[0, 0]);
    p00.extend_from_slice(&[0x00, 0x20]);
    p00.extend_from_slice(&m);
    assert!(import_swm(&p00).is_ok());
}

#[test]
fn rejects_garbage_without_panicking() {
    assert_eq!(import_swm(b"hello world").err(), Some(SwmError::NotSwm));
    let m = build_module();
    for cut in [10, 64, 70, m.len() - 1, m.len() - 20] {
        let _ = import_swm(&m[..cut]); // must not panic
    }
    // flipped bytes everywhere must never panic either
    for i in 0..m.len() {
        let mut x = m.clone();
        x[i] ^= 0xA5;
        let _ = import_swm(&x);
    }
}

#[test]
fn real_modules_import_and_play_identically() {
    let Ok(dir) = std::env::var("SIDBLOOM_SWM_DIR") else {
        eprintln!("SIDBLOOM_SWM_DIR not set, skipping");
        return;
    };
    let mut count = 0;
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
        let bytes = std::fs::read(&p).unwrap();
        let Ok(imp) = import_swm(&bytes) else {
            eprintln!("skip {name}: not a 3-voice SWM1 module");
            continue;
        };
        pack_song(&imp.song).unwrap_or_else(|e| panic!("{name}: cannot export: {e}"));
        assert_same(&imp.song, 50 * 20, &name);
        // and it must make sound
        let pcm = sidcore::audio::render_pcm(&imp.song, 6.0, sidcore::audio::SidModel::Mos8580);
        let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 500, "{name}: silent (peak {peak})");
        eprintln!("ok {name}: {} instruments, {} patterns, peak {peak}", imp.song.instruments.len(), imp.song.patterns.len());
        count += 1;
    }
    assert!(count > 0, "no modules found");
}
