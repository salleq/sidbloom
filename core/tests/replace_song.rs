//! Swapping in an identical song while playing (what the editor does ~120 ms after Play and after every edit)
//! must not change what is played.
use sidcore::engine::Engine;
use sidcore::project::demo_song;

#[test]
fn replacing_with_the_same_song_does_not_disturb_playback() {
    let song = demo_song();
    let mut plain = Engine::new(song.clone());
    let mut swapped = Engine::new(song.clone());
    for frame in 0..300 {
        if frame == 6 || frame == 61 || frame == 143 {
            swapped.replace_song(song.clone());
        }
        assert_eq!(plain.tick(), swapped.tick(), "frame {frame}");
    }
}
