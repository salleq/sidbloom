//! MIDI keyboard input for the desktop app (the system WebViews on macOS and Linux have no Web MIDI).
use midir::{Ignore, MidiInput, MidiInputConnection};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

/// What the UI needs to know about a MIDI message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MidiEvent {
    pub on: bool,
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
}

/// Parses a raw MIDI message. Only note on/off are interesting; a note on with velocity 0 is a note off.
pub fn parse(msg: &[u8]) -> Option<MidiEvent> {
    let [status, note, velocity, ..] = *msg else { return None };
    if note > 127 || velocity > 127 {
        return None;
    }
    let channel = status & 0x0F;
    match status & 0xF0 {
        0x90 => Some(MidiEvent { on: velocity > 0, channel, note, velocity }),
        0x80 => Some(MidiEvent { on: false, channel, note, velocity }),
        _ => None,
    }
}

#[derive(Default)]
pub struct MidiState(Mutex<Option<MidiInputConnection<()>>>);

fn client() -> Result<MidiInput, String> {
    let mut input = MidiInput::new("SIDbloom").map_err(|e| e.to_string())?;
    input.ignore(Ignore::SysexAndTime | Ignore::ActiveSense);
    Ok(input)
}

#[tauri::command]
pub fn midi_ports() -> Result<Vec<String>, String> {
    let input = client()?;
    Ok(input.ports().iter().filter_map(|p| input.port_name(p).ok()).collect())
}

#[tauri::command]
pub fn midi_open(app: AppHandle, state: State<MidiState>, name: String) -> Result<(), String> {
    let input = client()?;
    let port = input
        .ports()
        .into_iter()
        .find(|p| input.port_name(p).map(|n| n == name).unwrap_or(false))
        .ok_or_else(|| format!("MIDI input \"{name}\" not found"))?;
    let conn = input
        .connect(
            &port,
            "SIDbloom-in",
            move |_stamp, msg, _| {
                if let Some(ev) = parse(msg) {
                    let _ = app.emit("midi", ev);
                }
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    *state.0.lock().map_err(|e| e.to_string())? = Some(conn); // dropping the old connection closes it
    Ok(())
}

#[tauri::command]
pub fn midi_close(state: State<MidiState>) -> Result<(), String> {
    *state.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_and_off() {
        assert_eq!(parse(&[0x90, 69, 100]), Some(MidiEvent { on: true, channel: 0, note: 69, velocity: 100 }));
        assert_eq!(parse(&[0x83, 60, 64]), Some(MidiEvent { on: false, channel: 3, note: 60, velocity: 64 }));
    }

    #[test]
    fn note_on_with_zero_velocity_is_note_off() {
        assert_eq!(parse(&[0x95, 60, 0]).map(|e| e.on), Some(false));
    }

    #[test]
    fn ignores_everything_else() {
        assert_eq!(parse(&[0xB0, 7, 100]), None); // controller
        assert_eq!(parse(&[0xE0, 0, 64]), None); // pitch bend
        assert_eq!(parse(&[0xF8]), None); // clock
        assert_eq!(parse(&[0x90, 60]), None); // truncated
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0x90, 200, 10]), None); // data byte out of range
    }
}
