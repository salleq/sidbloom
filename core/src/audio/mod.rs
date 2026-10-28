//! Offline audio rendering: engine register writes -> reSID -> PCM.
use crate::engine::Engine;
use crate::model::Song;
use resid::{ChipModel, SamplingMethod, Sid};

pub use resid::ChipModel as SidModel;

pub const SAMPLE_RATE: u32 = 44_100;

/// Renders `seconds` of mono 16-bit audio at `SAMPLE_RATE`.
pub fn render_pcm(song: &Song, seconds: f64, model: ChipModel) -> Vec<i16> {
    let clock = song.clock_hz();
    let cycles_per_frame = (clock / song.frame_hz()).round() as u32;
    let frames = (seconds * song.frame_hz()).ceil() as usize;

    let mut sid = Sid::new(model);
    sid.set_sampling_parameters(SamplingMethod::ResampleFast, clock as u32, SAMPLE_RATE);
    let mut engine = Engine::new(song.clone());

    let mut pcm = Vec::with_capacity((seconds * SAMPLE_RATE as f64) as usize + 1024);
    let mut buf = [0i16; 2048];
    for _ in 0..frames {
        for (reg, val) in engine.tick() {
            sid.write(reg, val);
        }
        let mut delta = cycles_per_frame;
        while delta > 0 {
            let (n, rest) = sid.sample(delta, &mut buf, 1);
            pcm.extend_from_slice(&buf[..n]);
            delta = rest;
        }
    }
    pcm.truncate((seconds * SAMPLE_RATE as f64) as usize);
    pcm
}

/// Pull-based realtime renderer: call `fill` from an audio callback / worklet.
pub struct Renderer {
    engine: Engine,
    sid: Sid,
    cycles_per_frame: u32,
    queue: Vec<i16>,
    pos: usize,
    sample_rate: u32,
}

impl Renderer {
    pub fn new(song: Song, model: ChipModel, sample_rate: u32) -> Self {
        Renderer::starting_at(song, model, sample_rate, 0, 0)
    }

    pub fn starting_at(song: Song, model: ChipModel, sample_rate: u32, order_pos: usize, row: usize) -> Self {
        let clock = song.clock_hz();
        let cycles_per_frame = (clock / song.frame_hz()).round() as u32;
        let mut sid = Sid::new(model);
        sid.set_sampling_parameters(SamplingMethod::ResampleFast, clock as u32, sample_rate);
        Renderer { engine: Engine::start_at(song, order_pos, row), sid, cycles_per_frame, queue: Vec::new(), pos: 0, sample_rate }
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn positions(&self) -> [(usize, usize); 3] { self.engine.positions() }
    pub fn looped(&self) -> bool { self.engine.looped() }

    fn next_frame(&mut self) {
        self.queue.clear();
        self.pos = 0;
        for (reg, val) in self.engine.tick() {
            self.sid.write(reg, val);
        }
        let mut buf = [0i16; 2048];
        let mut delta = self.cycles_per_frame;
        while delta > 0 {
            let (n, rest) = self.sid.sample(delta, &mut buf, 1);
            self.queue.extend_from_slice(&buf[..n]);
            delta = rest;
        }
    }

    /// Fills `out` with mono samples in -1.0..1.0.
    pub fn fill(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            while self.pos >= self.queue.len() {
                self.next_frame();
            }
            *o = self.queue[self.pos] as f32 / 32768.0;
            self.pos += 1;
        }
    }
}

/// Canonical 44-byte-header PCM WAV, mono 16-bit.
pub fn wav_bytes(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut w = Vec::with_capacity(44 + pcm.len() * 2);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes()); // PCM
    w.extend_from_slice(&1u16.to_le_bytes()); // mono
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

pub fn render_wav(song: &Song, seconds: f64, model: ChipModel) -> Vec<u8> {
    wav_bytes(&render_pcm(song, seconds, model), SAMPLE_RATE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    /// Signal power at `freq` (Goertzel).
    fn power_at(pcm: &[i16], freq: f64) -> f64 {
        let w = 2.0 * std::f64::consts::PI * freq / SAMPLE_RATE as f64;
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in pcm {
            let s = x as f64 + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2) / pcm.len() as f64
    }

    fn single_voice_song(note: u8, wave: u8) -> Song {
        let mut s = Song::new();
        s.instruments = vec![Instrument::simple("t", wave, 0x00, 0xF0)];
        s.patterns = vec![Pattern::new(64)];
        s.patterns[0].rows[0] = Cell { note, inst: 1, ..Default::default() };
        s.orders = [vec![0], vec![1], vec![1]];
        s.patterns.push(Pattern::new(64)); // silent pattern for voices 2/3
        s.speed = 6;
        s
    }

    #[test]
    fn a4_triangle_peaks_at_440hz() {
        let song = single_voice_song(58, 0x10);
        let pcm = render_pcm(&song, 1.0, ChipModel::Mos8580);
        assert_eq!(pcm.len(), SAMPLE_RATE as usize);
        let tail = &pcm[SAMPLE_RATE as usize / 4..]; // skip attack
        let peak = tail.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 500, "output is silent (peak {peak})");
        let at = power_at(tail, 440.0);
        for off in [330.0, 392.0, 494.0, 523.25, 880.0] {
            assert!(at > 20.0 * power_at(tail, off), "440 Hz not dominant vs {off}");
        }
    }

    #[test]
    fn octave_up_doubles_frequency() {
        let a4 = render_pcm(&single_voice_song(58, 0x10), 1.0, ChipModel::Mos8580);
        let a5 = render_pcm(&single_voice_song(70, 0x10), 1.0, ChipModel::Mos8580);
        assert!(power_at(&a5[11025..], 880.0) > 20.0 * power_at(&a5[11025..], 440.0));
        assert!(power_at(&a4[11025..], 440.0) > 20.0 * power_at(&a4[11025..], 880.0));
    }

    #[test]
    fn streaming_renderer_matches_offline_render() {
        let song = single_voice_song(58, 0x10);
        let offline = render_pcm(&song, 0.5, ChipModel::Mos8580);
        let mut r = Renderer::new(song, ChipModel::Mos8580, SAMPLE_RATE);
        let mut out = vec![0f32; offline.len()];
        for chunk in out.chunks_mut(128) {
            r.fill(chunk);
        }
        for (a, b) in out.iter().zip(&offline) {
            assert_eq!((a * 32768.0).round() as i16, *b);
        }
        assert_eq!(r.positions()[0].0, 0);
    }

    #[test]
    fn start_at_begins_on_requested_row() {
        let mut song = single_voice_song(58, 0x10);
        song.patterns[0].rows[8] = Cell { note: 70, inst: 1, ..Default::default() };
        let mut e = Engine::start_at(song, 0, 8);
        e.tick();
        assert_eq!(e.positions()[0], (0, 8));
    }

    #[test]
    fn wav_header_is_valid() {
        let wav = wav_bytes(&[0, 1, -1, 2], 44_100);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(wav[4..8].try_into().unwrap()), 36 + 8);
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 8);
        assert_eq!(wav.len(), 44 + 8);
    }
}
