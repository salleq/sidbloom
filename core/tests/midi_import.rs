//! MIDI file import, with MIDI files built byte by byte.
mod common;
use common::assert_same;
use sidcore::audio::{render_pcm, SidModel, SAMPLE_RATE};
use sidcore::export::pack_song;
use sidcore::import::midi::{default_assignment, import_midi, list_parts, MidiError};
use sidcore::*;

fn vlq(mut v: u32) -> Vec<u8> {
    let mut out = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v > 0 {
        out.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.reverse();
    out
}

/// One track: events are (delta ticks, raw bytes).
fn track(events: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut body = vec![];
    for (d, e) in events {
        body.extend(vlq(*d));
        body.extend(e);
    }
    body.extend([0, 0xFF, 0x2F, 0]);
    let mut t = b"MTrk".to_vec();
    t.extend((body.len() as u32).to_be_bytes());
    t.extend(body);
    t
}

fn smf(format: u16, division: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
    let mut f = b"MThd".to_vec();
    f.extend(6u32.to_be_bytes());
    f.extend(format.to_be_bytes());
    f.extend((tracks.len() as u16).to_be_bytes());
    f.extend(division.to_be_bytes());
    for t in tracks {
        f.extend(t);
    }
    f
}

fn tempo(us: u32) -> (u32, Vec<u8>) {
    (0, vec![0xFF, 0x51, 3, (us >> 16) as u8, (us >> 8) as u8, us as u8])
}
fn name(n: &str) -> (u32, Vec<u8>) {
    let mut v = vec![0xFF, 0x03, n.len() as u8];
    v.extend(n.bytes());
    (0, v)
}
fn on(d: u32, ch: u8, key: u8) -> (u32, Vec<u8>) {
    (d, vec![0x90 | ch, key, 100])
}
fn off(d: u32, ch: u8, key: u8) -> (u32, Vec<u8>) {
    (d, vec![0x80 | ch, key, 0])
}

fn rows(song: &Song, ch: usize, pos: usize) -> &Vec<Cell> {
    &song.patterns[song.orders[ch][pos] as usize].rows
}

#[test]
fn quarter_notes_land_on_every_fourth_row_at_120_bpm() {
    // 480 ticks per quarter, 120 BPM: speed 6 (6.25 frames per row), rows = 120 ticks
    let t = track(&[name("Melody"), tempo(500_000), on(0, 0, 60), off(480, 0, 60), on(0, 0, 62), off(240, 0, 62)]);
    let imp = import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap();
    let s = &imp.song;
    assert_eq!(s.title, "Melody");
    assert_eq!(s.speed, 6);
    let r = rows(s, 0, 0);
    assert_eq!((r[0].note, r[4].note), (49, 51), "C4 = note 49, D4 = note 51"); // MIDI 60 - 11
    assert_eq!(r[0].inst, 1);
    assert_eq!(r[6].note, NOTE_OFF, "D4 lasts an eighth note: off two rows later");
    assert_eq!(r[3].note, 0, "legato: no note-off between touching notes");
    // voices without a part are silent
    assert!(rows(s, 1, 0).iter().all(|c| c.note == 0));
    pack_song(s).unwrap();
}

#[test]
fn tempo_sets_the_speed() {
    let t = track(&[tempo(666_667), on(0, 0, 60), off(480, 0, 60)]); // 90 BPM
    let imp = import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap();
    assert_eq!(imp.song.speed, 8);
    let t = track(&[tempo(250_000), on(0, 0, 60), off(480, 0, 60)]); // 240 BPM
    assert_eq!(import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap().song.speed, 3);
}

#[test]
fn chords_keep_the_highest_note_and_say_so() {
    let t = track(&[on(0, 0, 60), on(0, 0, 64), on(0, 0, 67), off(480, 0, 60), off(0, 0, 64), off(0, 0, 67)]);
    let imp = import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap();
    assert_eq!(rows(&imp.song, 0, 0)[0].note, 67 - 11);
    assert!(imp.warnings.iter().any(|w| w.contains("2 notes were left out")), "{:?}", imp.warnings);
}

#[test]
fn drums_use_a_noise_instrument_and_running_status_works() {
    // channel 10 (index 9): kick, then snare using running status (no status byte)
    let mut ev = vec![(0, vec![0x99, 36, 100]), (120, vec![0x99, 36, 0])];
    ev.push((0, vec![38, 100])); // running status note on
    ev.push((120, vec![38, 0])); // running status note on, velocity 0 = off
    let imp = import_midi(&smf(0, 480, &[track(&ev)]), [Some(0), None, None]).unwrap();
    let r = rows(&imp.song, 0, 0);
    assert_eq!(r[0].inst, 3);
    assert_eq!(imp.song.instruments[2].wave_table[0].wave, 0x80, "noise");
    assert_ne!(r[0].note, r[1].note, "kick and snare get different noise pitches");
}

#[test]
fn parts_are_listed_and_the_busiest_three_are_preselected() {
    let a = track(&[name("A"), on(0, 0, 60), off(10, 0, 60)]);
    let b = track(&[name("B"), on(0, 1, 40), off(10, 1, 40), on(10, 1, 41), off(10, 1, 41)]);
    let c = track(&[name("C"), on(0, 2, 70), off(10, 2, 70), on(10, 2, 71), off(10, 2, 71), on(10, 2, 72), off(10, 2, 72)]);
    let d = track(&[name("D"), on(0, 3, 50), off(10, 3, 50), on(10, 3, 50), off(10, 3, 50)]);
    let bytes = smf(1, 480, &[a, b, c, d]);
    let parts = list_parts(&bytes).unwrap();
    assert_eq!(parts.iter().map(|p| (p.name.as_str(), p.notes)).collect::<Vec<_>>(), [("A", 1), ("B", 2), ("C", 3), ("D", 2)]);
    assert_eq!((parts[2].min, parts[2].max), (70, 72));
    let pick = default_assignment(&parts);
    assert_eq!(pick, [Some(1), Some(2), Some(3)]);
}

#[test]
fn a_type_0_file_is_split_into_channels() {
    let t = track(&[on(0, 0, 60), on(0, 1, 40), off(100, 0, 60), off(0, 1, 40)]);
    let parts = list_parts(&smf(0, 480, &[t])).unwrap();
    assert_eq!(parts.iter().map(|p| p.channel).collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn long_songs_are_split_into_patterns_and_identical_ones_are_shared() {
    // the same one-bar phrase 16 times
    let mut ev = vec![];
    for _ in 0..16 {
        ev.push(on(0, 0, 60));
        ev.push(off(1920, 0, 60));
    }
    let imp = import_midi(&smf(0, 480, &[track(&ev)]), [Some(0), None, None]).unwrap();
    assert!(imp.song.orders[0].len() >= 8);
    assert!(imp.song.patterns.len() <= 6, "repeats share patterns, got {}", imp.song.patterns.len());
    pack_song(&imp.song).unwrap();
    assert_same(&imp.song, 300, "midi import");
}

#[test]
fn out_of_range_notes_are_shifted_by_octaves() {
    let t = track(&[on(0, 0, 5), off(100, 0, 5), on(0, 0, 127), off(100, 0, 127)]);
    let imp = import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap();
    for c in rows(&imp.song, 0, 0).iter().filter(|c| c.note != 0 && c.note != NOTE_OFF) {
        assert!((1..=95).contains(&c.note));
    }
    assert!(imp.warnings.iter().any(|w| w.contains("octave")));
}

#[test]
fn the_imported_notes_sound_at_the_right_pitch() {
    // A4 (MIDI 69) held for four beats = 2 s at 120 BPM
    let t = track(&[on(0, 0, 69), off(1920, 0, 69)]);
    let mut imp = import_midi(&smf(0, 480, &[t]), [Some(0), None, None]).unwrap();
    imp.song.instruments[0].wave_table = vec![WaveStep { wave: 0x10, semis: 0 }]; // triangle: a clean tone
    imp.song.instruments[0].pulse_speed = 0;
    let pcm = render_pcm(&imp.song, 1.5, SidModel::Mos8580);
    let seg = &pcm[SAMPLE_RATE as usize / 4..];
    let power = |f: f64| {
        let w = 2.0 * std::f64::consts::PI * f / SAMPLE_RATE as f64;
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in seg {
            let s = x as f64 + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2
    };
    assert!(power(440.0) > 20.0 * power(466.16) && power(440.0) > 20.0 * power(415.3), "A4 should dominate");
    assert!(seg.iter().map(|s| s.unsigned_abs()).max().unwrap() > 500, "silent");
}

#[test]
fn bad_files_give_errors_not_panics() {
    assert_eq!(import_midi(b"hello", [None; 3]).err(), Some(MidiError::NotMidi));
    assert_eq!(list_parts(&smf(0, 0x8000 | 25, &[track(&[])])).err(), Some(MidiError::Unsupported("SMPTE time division")));
    assert_eq!(list_parts(&smf(0, 480, &[track(&[(0, vec![0xFF, 0x51, 3, 7, 0xA1, 0x20])])])).err(), Some(MidiError::NoNotes));
    let good = smf(1, 480, &[track(&[tempo(500_000), on(0, 0, 60), off(480, 0, 60), on(0, 0, 64), off(480, 0, 64)])]);
    for cut in 0..good.len() {
        let _ = import_midi(&good[..cut], [Some(0), None, None]);
        let _ = list_parts(&good[..cut]);
    }
    for i in 0..good.len() {
        for flip in [0xFFu8, 0x80, 0x01] {
            let mut x = good.clone();
            x[i] ^= flip;
            let _ = import_midi(&x, [Some(0), Some(1), Some(2)]);
        }
    }
    // a part index that does not exist is simply an empty voice
    assert!(import_midi(&good, [Some(99), None, None]).is_ok());
}
