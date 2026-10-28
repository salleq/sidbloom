//! Tick-based playback engine. One `tick()` = one video frame (50/60 Hz) and returns the
//! SID register writes for that frame. The 6502 player mirrors this logic.
use crate::model::*;

pub type RegWrite = (u8, u8);

/// Frequency register value for a note (1 = C-0 ... 95 = B-7), A-4 (note 58) = 440 Hz.
pub fn note_freq(note: i32, clock_hz: f64) -> u16 {
    let hz = 440.0 * 2f64.powf((note as f64 - 58.0) / 12.0);
    (hz * 16_777_216.0 / clock_hz).round().clamp(0.0, 65535.0) as u16
}

/// Saturating signed add, mirrored exactly by the 6502 player.
pub fn add_clamped(f: u16, d: i16) -> u16 {
    (f as i32 + d as i32).clamp(0, 65535) as u16
}

#[derive(Default, Clone)]
struct Voice {
    order_pos: usize,
    row: usize,
    inst: Option<usize>,
    note: i32,
    gate: bool,
    freq_offset: i16, // slide accumulator (wraps, like the 6502 player)
    wave_pos: usize,
    fx: u8,
    fx_param: u8,
    vib_phase: u8,
    pw: u16,
    last_ctrl: u8,
    wrapped: bool,
}

pub struct Engine {
    song: Song,
    voices: [Voice; CHANNELS],
    speed: u8,
    tick_in_row: u8,
    freq_table: Vec<u16>,
    first: bool,
}

impl Engine {
    pub fn new(song: Song) -> Self {
        let clock = song.clock_hz();
        let freq_table = (0..=96).map(|n| note_freq(n, clock)).collect();
        let speed = song.speed.max(1);
        Engine { song, voices: Default::default(), speed, tick_in_row: 0, freq_table, first: true }
    }

    pub fn song(&self) -> &Song { &self.song }

    /// True once every channel has wrapped its order list at least once.
    pub fn looped(&self) -> bool { self.voices.iter().all(|v| v.wrapped) }

    fn cell_at(&self, ch: usize, order_pos: usize, row: usize) -> Cell {
        let order = &self.song.orders[ch];
        let pat = order.get(order_pos).and_then(|&i| self.song.patterns.get(i as usize));
        pat.and_then(|p| p.rows.get(row)).copied().unwrap_or_default()
    }

    fn pattern_len(&self, ch: usize, order_pos: usize) -> usize {
        let order = &self.song.orders[ch];
        order.get(order_pos).and_then(|&i| self.song.patterns.get(i as usize)).map_or(1, |p| p.rows.len().max(1))
    }

    fn next_pos(&self, ch: usize) -> (usize, usize, bool) {
        let v = &self.voices[ch];
        let (mut op, mut row) = (v.order_pos, v.row + 1);
        let mut wrapped = false;
        if row >= self.pattern_len(ch, op) {
            row = 0;
            op += 1;
            if op >= self.song.orders[ch].len() { op = 0; wrapped = true; }
        }
        (op, row, wrapped)
    }

    fn trigger_row(&mut self, ch: usize) {
        let v = &self.voices[ch];
        let cell = self.cell_at(ch, v.order_pos, v.row);
        let n_inst = self.song.instruments.len();
        let v = &mut self.voices[ch];
        if cell.inst != 0 && (cell.inst as usize) <= n_inst { v.inst = Some(cell.inst as usize - 1); }
        if cell.note == NOTE_OFF {
            v.gate = false;
        } else if cell.note != 0 && v.inst.is_some() {
            v.note = cell.note as i32;
            v.gate = true;
            v.freq_offset = 0;
            v.wave_pos = 0;
            v.vib_phase = 0;
            v.pw = self.song.instruments[v.inst.unwrap()].pulse_width;
        }
        // effects persist per row; an empty fx column clears them
        v.fx = cell.fx;
        v.fx_param = cell.fx_param;
        if cell.fx == fx::SET_SPEED && cell.fx_param > 0 { self.speed = cell.fx_param; }
    }

    pub fn tick(&mut self) -> Vec<RegWrite> {
        let mut w = Vec::with_capacity(32);
        if self.first {
            self.first = false;
            for ch in 0..CHANNELS { self.trigger_row(ch); }
        } else if self.tick_in_row == 0 {
            for ch in 0..CHANNELS {
                let (op, row, wrapped) = self.next_pos(ch);
                let v = &mut self.voices[ch];
                v.order_pos = op;
                v.row = row;
                v.wrapped |= wrapped;
            }
            for ch in 0..CHANNELS { self.trigger_row(ch); }
        }

        let preempt = self.tick_in_row + 1 == self.speed;
        for ch in 0..CHANNELS {
            // gate-off one tick before a new note so the ADSR retriggers
            let retrigger = preempt && {
                let (op, row, _) = self.next_pos(ch);
                let c = self.cell_at(ch, op, row);
                c.note != 0
            };
            self.render_voice(ch, retrigger, &mut w);
        }

        let f = self.song.filter;
        w.push((21, (f.cutoff & 7) as u8));
        w.push((22, (f.cutoff >> 3) as u8));
        w.push((23, (f.resonance << 4) | (f.routing & 7)));
        w.push((24, (f.mode & 0x70) | (f.volume & 15)));

        self.tick_in_row += 1;
        if self.tick_in_row >= self.speed { self.tick_in_row = 0; }
        w
    }

    fn render_voice(&mut self, ch: usize, retrigger: bool, w: &mut Vec<RegWrite>) {
        let tick = self.tick_in_row;
        let base = (ch * 7) as u8;
        let Some(ii) = self.voices[ch].inst else {
            return;
        };
        let inst = self.song.instruments[ii].clone();
        let v = &mut self.voices[ch];

        // wave table step
        let steps = &inst.wave_table;
        let step = if steps.is_empty() { WaveStep { wave: 0x40, semis: 0 } } else { steps[v.wave_pos.min(steps.len() - 1)] };
        if !steps.is_empty() {
            if v.wave_pos + 1 < steps.len() { v.wave_pos += 1; }
            else if let Some(l) = inst.wave_loop { v.wave_pos = (l as usize).min(steps.len() - 1); }
        }

        // effects
        let mut semis = step.semis as i32;
        match v.fx {
            fx::ARPEGGIO if v.fx_param != 0 => {
                semis += match tick % 3 { 0 => 0, 1 => (v.fx_param >> 4) as i32, _ => (v.fx_param & 15) as i32 };
            }
            fx::SLIDE_UP if tick > 0 => v.freq_offset = v.freq_offset.wrapping_add(v.fx_param as i16),
            fx::SLIDE_DOWN if tick > 0 => v.freq_offset = v.freq_offset.wrapping_sub(v.fx_param as i16),
            _ => {}
        }
        let mut freq = add_clamped(self.freq_table[(v.note + semis).clamp(0, 96) as usize], v.freq_offset);
        if v.fx == fx::VIBRATO {
            let (speed, depth) = (v.fx_param >> 4, (v.fx_param & 15) as i32);
            v.vib_phase = v.vib_phase.wrapping_add(speed.max(1) * 4);
            // triangle LFO -64..63
            let p = v.vib_phase as i32;
            let tri = if p < 128 { p - 64 } else { 192 - p };
            freq = add_clamped(freq, (tri * depth / 16) as i16);
        }

        let gate = v.gate && !retrigger;
        let ctrl = step.wave | gate as u8;
        w.push((base, freq as u8));
        w.push((base + 1, (freq >> 8) as u8));
        w.push((base + 2, v.pw as u8));
        w.push((base + 3, (v.pw >> 8) as u8 & 0x0F));
        w.push((base + 5, inst.attack_decay));
        w.push((base + 6, inst.sustain_release));
        w.push((base + 4, ctrl));
        v.last_ctrl = ctrl;
    }
}

/// Render `frames` ticks into a flat per-frame register log (for tests and export validation).
pub fn render_log(song: &Song, frames: usize) -> Vec<Vec<RegWrite>> {
    let mut e = Engine::new(song.clone());
    (0..frames).map(|_| e.tick()).collect()
}
