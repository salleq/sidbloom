// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(midi::MidiState::default())
        .invoke_handler(tauri::generate_handler![midi::midi_ports, midi::midi_open, midi::midi_close])
        .run(tauri::generate_context!())
        .expect("error while running SIDbloom");
}
