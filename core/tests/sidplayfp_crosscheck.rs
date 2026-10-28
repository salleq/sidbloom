//! Plays our exported .sid with the independent `sidplayfp` player (skipped if not installed)
//! and checks the result is audible and dominated by the notes we wrote.
use sidcore::export::export_sid;
use sidcore::project::demo_song;
use std::process::Command;

fn power_at(pcm: &[i16], sr: f64, freq: f64) -> f64 {
    let w = 2.0 * std::f64::consts::PI * freq / sr;
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &x in pcm {
        let s = x as f64 + 2.0 * w.cos() * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2) / pcm.len() as f64
}

#[test]
fn sidplayfp_plays_our_sid() {
    let dir = std::env::temp_dir().join("sidbloom_xcheck");
    std::fs::create_dir_all(&dir).unwrap();
    let sid = dir.join("demo.sid");
    std::fs::write(&sid, export_sid(&demo_song(), false).unwrap()).unwrap();
    let wav = dir.join("out.wav");
    let Ok(out) = Command::new("sidplayfp").arg(format!("-w{}", wav.display())).arg("-t4").arg(&sid).output() else {
        eprintln!("sidplayfp not installed, skipping");
        return;
    };
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "sidplayfp failed: {text}");
    assert!(text.contains("SIDbloom demo"), "title not read from header: {text}");

    let bytes = std::fs::read(&wav).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    let ch = u16::from_le_bytes([bytes[22], bytes[23]]) as usize;
    let sr = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as f64;
    let pcm: Vec<i16> = bytes[44..].chunks_exact(2 * ch).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    assert!(pcm.len() as f64 > 3.5 * sr);
    assert!(pcm.iter().map(|s| s.unsigned_abs()).max().unwrap() > 1000, "silent");
    let seg = &pcm[(sr / 2.0) as usize..(sr * 3.0) as usize];
    assert!(power_at(seg, sr, 440.0) > 10.0 * power_at(seg, sr, 300.0), "A-4 not dominant");
}
