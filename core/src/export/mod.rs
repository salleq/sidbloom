//! C64 export: packs a `Song` for the 6502 player and wraps it as `.sid` (PSID v2) or `.prg`.
pub mod embed;
pub mod pack;
pub mod psid;

pub use pack::{pack_song, PackError, DATA_BASE, INIT_ADDR, LOAD_ADDR, PLAY_ADDR};
pub use embed::{extract as extract_project, EmbedError};
pub use psid::{export_prg, export_sid, project_fits};
