//! An exported .sid / .prg can be opened again, and it still plays like before.
mod common;
use common::{assert_same, prg_log};
use sidcore::engine::render_log;
use sidcore::export::{export_prg, export_sid, extract_project, project_fits, EmbedError};
use sidcore::project::demo_song;
use sidcore::*;

#[test]
fn sid_round_trip_demo() {
    let song = demo_song();
    let sid = export_sid(&song, true).unwrap();
    assert_eq!(extract_project(&sid).unwrap(), song);
    assert!(project_fits(&song));
}

#[test]
fn prg_round_trip_and_still_boots() {
    let song = demo_song();
    let prg = export_prg(&song).unwrap();
    assert_eq!(extract_project(&prg).unwrap(), song);
    assert_eq!(prg_log(&prg, 120), render_log(&song, 120), "trailer must not disturb playback");
}

#[test]
fn player_memory_image_is_unaffected_by_the_trailer() {
    let mut song = demo_song();
    song.chords = vec![Chord { notes: vec![0, 4, 7], hold: false }];
    song.instruments[0].default_chord = 1;
    let _ = export_sid(&song, false).unwrap();
    assert_same(&song, 200, "with trailer");
    assert_eq!(extract_project(&export_sid(&song, false).unwrap()).unwrap(), song);
}

#[test]
fn files_without_project_data_are_reported_as_such() {
    assert_eq!(extract_project(b"PSID\x00\x02not ours"), Err(EmbedError::NoProject));
    assert_eq!(extract_project(b""), Err(EmbedError::NoProject));
    assert_eq!(extract_project(b"SBLM1"), Err(EmbedError::NoProject));
}

#[test]
fn damaged_data_is_an_error_never_a_panic() {
    let sid = export_sid(&demo_song(), true).unwrap();
    // truncated anywhere
    for cut in (0..sid.len()).step_by(37) {
        let _ = extract_project(&sid[..cut]);
    }
    // flipped bytes in the payload / trailer
    let n = sid.len();
    for i in (n.saturating_sub(600)..n).step_by(3) {
        let mut x = sid.clone();
        x[i] ^= 0x5A;
        let _ = extract_project(&x);
    }
    // absurd length field
    let mut x = sid.clone();
    x[n - 9..n - 5].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(extract_project(&x), Err(EmbedError::Corrupt(_))));
    // valid deflate of valid JSON that is not a usable song
    let mut bad = demo_song();
    bad.orders[0].clear();
    let tail = sidcore::export::embed::payload(&bad, 0x1000).unwrap();
    assert!(matches!(extract_project(&tail), Err(EmbedError::Corrupt(_))));
}

#[test]
fn huge_songs_skip_the_embedding_instead_of_overflowing_memory() {
    let mut song = Song::new();
    // lots of distinct, incompressible-ish pattern data
    let mut x = 12345u32;
    song.patterns = (0..128)
        .map(|_| Pattern {
            rows: (0..64)
                .map(|_| {
                    x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                    Cell { note: (x >> 24) as u8 % 90 + 1, inst: 1, fx: fx::VIBRATO, fx_param: (x >> 16) as u8 }
                })
                .collect(),
        })
        .collect();
    song.orders = [(0..128).collect(), (0..128).collect(), (0..128).collect()];
    let sid = export_sid(&song, false).unwrap();
    let loaded_end = 0x1000 + sid.len() - 0x7C - 2;
    assert!(loaded_end <= 0xD000, "export must still fit below the I/O area: ends at {loaded_end:#x}");
    // whether or not it fit, extraction must agree with project_fits
    assert_eq!(extract_project(&sid).is_ok(), project_fits(&song));
}
