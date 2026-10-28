//! The `.prg` export must start by itself (BASIC `SYS` + IRQ) and play exactly like the engine.
mod common;
use common::prg_log;
use sidcore::engine::render_log;
use sidcore::export::export_prg;
use sidcore::project::demo_song;

#[test]
fn prg_boots_and_plays_like_the_engine_pal() {
    let song = demo_song();
    let got = prg_log(&export_prg(&song).unwrap(), 300);
    let expected = render_log(&song, 300);
    for (i, (e, g)) in expected.iter().zip(&got).enumerate() {
        assert_eq!(e, g, "frame {i}");
    }
}

#[test]
fn prg_uses_ntsc_timer_for_ntsc_songs() {
    let mut song = demo_song();
    song.pal = false;
    let got = prg_log(&export_prg(&song).unwrap(), 50);
    assert_eq!(got, render_log(&song, 50));
}
