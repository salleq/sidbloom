//! Lets an exported `.sid` / `.prg` be opened again for editing: the project is deflated and appended to the
//! file as `[deflate(JSON)][length u32 LE]["SBLM1"]`. C64 players and SID players load these bytes as inert
//! extra data after the music; only SIDbloom looks at them.
use crate::model::Song;
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec_with_limit};

const MAGIC: &[u8; 5] = b"SBLM1";
const TRAILER: usize = 4 + MAGIC.len();
const MAX_JSON: usize = 16 * 1024 * 1024;
/// Highest address embedded data may reach: the C64 I/O area starts here.
pub const MEMORY_LIMIT: usize = 0xD000;

#[derive(Debug, PartialEq, Eq)]
pub enum EmbedError {
    /// A `.sid`/`.prg` (or other file) without SIDbloom project data.
    NoProject,
    Corrupt(String),
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedError::NoProject => write!(f, "this file contains no SIDbloom project (it was not exported by SIDbloom, or by an older version)"),
            EmbedError::Corrupt(why) => write!(f, "the embedded SIDbloom project is damaged: {why}"),
        }
    }
}
impl std::error::Error for EmbedError {}

/// The bytes to append to an export; `None` if they would not fit below the I/O area when loaded.
/// `end_address` is the address just past the music data when loaded on a C64.
pub fn payload(song: &Song, end_address: usize) -> Option<Vec<u8>> {
    let json = serde_json::to_vec(song).ok()?;
    let mut out = compress_to_vec(&json, 9);
    let len = out.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(MAGIC);
    (end_address + out.len() <= MEMORY_LIMIT).then_some(out)
}

/// Does the project fit into the export of this song?
pub fn fits(song: &Song, end_address: usize) -> bool {
    payload(song, end_address).is_some()
}

/// Reads the project back out of a `.sid` / `.prg` exported by SIDbloom.
pub fn extract(bytes: &[u8]) -> Result<Song, EmbedError> {
    if bytes.len() < TRAILER || &bytes[bytes.len() - MAGIC.len()..] != MAGIC {
        return Err(EmbedError::NoProject);
    }
    let len_pos = bytes.len() - TRAILER;
    let len = u32::from_le_bytes(bytes[len_pos..len_pos + 4].try_into().unwrap()) as usize;
    let start = len_pos.checked_sub(len).ok_or_else(|| EmbedError::Corrupt("length is larger than the file".into()))?;
    let json = decompress_to_vec_with_limit(&bytes[start..len_pos], MAX_JSON)
        .map_err(|e| EmbedError::Corrupt(format!("{e:?}")))?;
    let song: Song = serde_json::from_slice(&json).map_err(|e| EmbedError::Corrupt(e.to_string()))?;
    sane(&song).map_err(EmbedError::Corrupt)?;
    Ok(song)
}

/// Checks that a song loaded from outside cannot break the editor or the engine.
pub fn sane(song: &Song) -> Result<(), String> {
    if song.patterns.is_empty() || song.instruments.is_empty() {
        return Err("no patterns or instruments".into());
    }
    for (ch, o) in song.orders.iter().enumerate() {
        if o.is_empty() {
            return Err(format!("voice {} has an empty order list", ch + 1));
        }
        if let Some(&p) = o.iter().find(|&&p| p as usize >= song.patterns.len()) {
            return Err(format!("voice {} refers to missing pattern {p}", ch + 1));
        }
    }
    if song.patterns.iter().any(|p| p.rows.is_empty()) {
        return Err("a pattern has no rows".into());
    }
    if song.instruments.iter().any(|i| i.wave_table.is_empty()) {
        return Err("an instrument has no wave table".into());
    }
    Ok(())
}
