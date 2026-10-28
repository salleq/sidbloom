//! WebAssembly bindings: the UI keeps the song as JSON and calls these for playback and export.
use sidcore::audio::{Renderer, SidModel};
use sidcore::{export, project, Song};
use wasm_bindgen::prelude::*;

fn parse(json: &str) -> Result<Song, JsError> {
    project::from_json(json).map_err(|e| JsError::new(&format!("invalid song: {e}")))
}

fn model(is_8580: bool) -> SidModel {
    if is_8580 { SidModel::Mos8580 } else { SidModel::Mos6581 }
}

/// A fresh song as JSON.
#[wasm_bindgen]
pub fn new_song() -> String { project::to_json(&Song::new()) }

#[wasm_bindgen]
pub fn demo_song() -> String { project::to_json(&project::demo_song()) }

/// Returns an error string if the song cannot be exported to a C64 (limits exceeded), else "".
#[wasm_bindgen]
pub fn validate(song_json: &str) -> Result<String, JsError> {
    let song = parse(song_json)?;
    Ok(match export::pack_song(&song) {
        Ok(_) => String::new(),
        Err(e) => format!("{e}"),
    })
}

#[wasm_bindgen]
pub fn export_sid(song_json: &str, is_8580: bool) -> Result<Vec<u8>, JsError> {
    export::export_sid(&parse(song_json)?, is_8580).map_err(|e| JsError::new(&format!("{e}")))
}

#[wasm_bindgen]
pub fn export_prg(song_json: &str) -> Result<Vec<u8>, JsError> {
    export::export_prg(&parse(song_json)?).map_err(|e| JsError::new(&format!("{e}")))
}

/// Realtime player driven from an AudioWorklet.
#[wasm_bindgen]
pub struct Player {
    inner: Renderer,
}

#[wasm_bindgen]
impl Player {
    #[wasm_bindgen(constructor)]
    pub fn new(song_json: &str, is_8580: bool, sample_rate: u32, order_pos: usize, row: usize) -> Result<Player, JsError> {
        let song = parse(song_json)?;
        Ok(Player { inner: Renderer::starting_at(song, model(is_8580), sample_rate, order_pos, row) })
    }

    /// Swaps in an edited song while playing (keeps the playback position).
    pub fn update(&mut self, song_json: &str) -> Result<(), JsError> {
        self.inner.update_song(parse(song_json)?);
        Ok(())
    }

    /// Fills `out` with mono samples.
    pub fn fill(&mut self, out: &mut [f32]) { self.inner.fill(out) }

    /// Flat [order0,row0, order1,row1, order2,row2].
    pub fn positions(&self) -> Vec<u32> {
        self.inner.positions().iter().flat_map(|&(o, r)| [o as u32, r as u32]).collect()
    }
}

/// Imports a SID-Wizard 1.x module. Returns JSON `{"song": ..., "warnings": [...]}`.
#[wasm_bindgen]
pub fn import_swm(bytes: &[u8]) -> Result<String, JsError> {
    let imp = sidcore::import::swm::import_swm(bytes).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(format!(
        "{{\"song\":{},\"warnings\":{}}}",
        serde_json::to_string(&imp.song).map_err(|e| JsError::new(&e.to_string()))?,
        serde_json::to_string(&imp.warnings).map_err(|e| JsError::new(&e.to_string()))?
    ))
}
