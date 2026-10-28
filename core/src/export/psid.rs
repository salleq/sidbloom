//! PSID v2 (`.sid`) and `.prg` writers. PSID spec: https://hvsc.c64.org/download/C64Music/DOCUMENTS/SID_file_format.txt
use super::pack::*;
use crate::model::Song;

fn put_str(buf: &mut [u8], s: &str) {
    // PSID text fields are 32 bytes, Latin-1, NUL padded; keep the last byte NUL.
    for (d, c) in buf.iter_mut().zip(s.chars().take(31)) {
        *d = if (c as u32) < 256 { c as u8 } else { b'?' };
    }
}

/// `sid_8580`: pick the 8580 model flag instead of 6581.
pub fn export_sid(song: &Song, sid_8580: bool) -> Result<Vec<u8>, PackError> {
    let image = pack_song(song)?;
    let mut h = vec![0u8; 0x7C];
    h[0..4].copy_from_slice(b"PSID");
    h[4..6].copy_from_slice(&2u16.to_be_bytes());
    h[6..8].copy_from_slice(&0x7Cu16.to_be_bytes());
    // load address 0: it is stored as the first two (little-endian) bytes of the data
    h[10..12].copy_from_slice(&INIT_ADDR.to_be_bytes());
    h[12..14].copy_from_slice(&PLAY_ADDR.to_be_bytes());
    h[14..16].copy_from_slice(&1u16.to_be_bytes()); // songs
    h[16..18].copy_from_slice(&1u16.to_be_bytes()); // start song
    // speed bits 0 = vertical blank (50/60 Hz per clock flag)
    put_str(&mut h[0x16..0x36], &song.title);
    put_str(&mut h[0x36..0x56], &song.author);
    put_str(&mut h[0x56..0x76], &song.released);
    let clock: u16 = if song.pal { 0b01 } else { 0b10 };
    let model: u16 = if sid_8580 { 0b10 } else { 0b01 };
    h[0x76..0x78].copy_from_slice(&(clock << 2 | model << 4).to_be_bytes());
    let mut out = h;
    out.extend_from_slice(&LOAD_ADDR.to_le_bytes());
    out.extend_from_slice(&image);
    Ok(out)
}

/// A C64 `.prg` (load address + image). `SYS` it after loading: it needs a tiny
/// caller, so this export is meant for tools/loaders that call init/play themselves.
pub fn export_prg(song: &Song) -> Result<Vec<u8>, PackError> {
    let image = pack_song(song)?;
    let mut out = LOAD_ADDR.to_le_bytes().to_vec();
    out.extend_from_slice(&image);
    Ok(out)
}
