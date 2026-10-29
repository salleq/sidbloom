// Ready-made instruments. Drums are wave-table tricks: one step per frame, a semitone offset per step, and the
// SID's noise / triangle waveforms; `note` is a good pitch to play (and preview) them at.
import type { Chord, Instrument, WaveStep } from "./model";

export interface Preset { category: string; note: number; instrument: Instrument; chord?: Chord }

const TRI = 0x10, SAW = 0x20, PULSE = 0x40, NOISE = 0x80, OFF = 0x00;
const steps = (...s: [number, number][]): WaveStep[] => s.map(([wave, semis]) => ({ wave, semis }));
const ins = (name: string, ad: number, sr: number, wave_table: WaveStep[], o: Partial<Instrument> = {}): Instrument => ({
  name, attack_decay: ad, sustain_release: sr, pulse_width: 0x800, wave_table, wave_loop: null,
  pulse_speed: 0, pulse_min: 0, pulse_max: 0x0fff, default_chord: 0, ...o,
});
// note numbers: C-0 = 1, so C-2 = 25, C-3 = 37, C-4 = 49
const drop = (wave: number, from: number, ...rest: number[]) => steps(...[from, ...rest].map((s): [number, number] => [wave, s]));

export const LIBRARY: Preset[] = [
  { category: "Drums", note: 25, instrument: ins("Kick", 0x05, 0x05, [{ wave: NOISE, semis: 14 }, ...drop(TRI, 24, 20, 16, 12, 9, 6, 4, 2, 1, 0)]) },
  { category: "Drums", note: 25, instrument: ins("Kick punchy", 0x04, 0x04, steps([NOISE, 12], [PULSE, 18], [PULSE, 12], [PULSE, 7], [PULSE, 3], [PULSE, 0]), { pulse_width: 0x400 }) },
  { category: "Drums", note: 37, instrument: ins("Snare", 0x06, 0x06, steps([PULSE, 7], [NOISE, 14], [NOISE, 10], [NOISE, 7], [NOISE, 5], [NOISE, 3]), { pulse_width: 0x600 }) },
  { category: "Drums", note: 44, instrument: ins("Snare tight", 0x04, 0x04, steps([NOISE, 12], [NOISE, 6], [NOISE, 0])) },
  { category: "Drums", note: 52, instrument: ins("Clap", 0x08, 0x08, steps([NOISE, 0], [NOISE, 0], [OFF, 0], [NOISE, 0], [NOISE, 0], [OFF, 0], [NOISE, 0], [NOISE, 0], [NOISE, 0])) },
  { category: "Drums", note: 70, instrument: ins("Hihat closed", 0x02, 0x02, steps([NOISE, 0])) },
  { category: "Drums", note: 70, instrument: ins("Hihat open", 0x07, 0x07, steps([NOISE, 0])) },
  { category: "Drums", note: 70, instrument: ins("Hihat pedal", 0x03, 0x03, steps([NOISE, 0], [NOISE, -4])) },
  { category: "Drums", note: 62, instrument: ins("Hihat sizzle", 0x05, 0x05, steps([NOISE, 4], [NOISE, 0], [NOISE, 4], [NOISE, 0], [NOISE, 4], [NOISE, 0])) },
  { category: "Drums", note: 33, instrument: ins("Tom", 0x07, 0x07, drop(TRI, 12, 9, 7, 5, 3, 2, 1, 0)) },
  { category: "Drums", note: 66, instrument: ins("Crash", 0x0a, 0x0a, steps([NOISE, 4], [NOISE, 0], [NOISE, 3], [NOISE, 0])) },
  { category: "Drums", note: 62, instrument: ins("Ride", 0x08, 0x08, steps([NOISE, 6], [NOISE, 0], [PULSE, 24], [NOISE, 0]), { pulse_width: 0x300 }) },
  { category: "Bass", note: 25, instrument: ins("Bass pulse", 0x05, 0x85, steps([PULSE, 0]), { pulse_width: 0x500, pulse_speed: 10, pulse_min: 0x300, pulse_max: 0xb00 }) },
  { category: "Bass", note: 25, instrument: ins("Bass saw", 0x04, 0x84, steps([SAW, 0])) },
  { category: "Bass", note: 25, instrument: ins("Bass sub", 0x08, 0xa8, steps([TRI, 0])) },
  { category: "Bass", note: 25, instrument: ins("Bass pluck", 0x06, 0x06, steps([PULSE, 12], [PULSE, 0]), { pulse_width: 0x400 }) },
  { category: "Lead", note: 49, instrument: ins("Lead pulse", 0x08, 0xa8, steps([PULSE, 0]), { pulse_speed: 12, pulse_min: 0x300, pulse_max: 0xd00 }) },
  { category: "Lead", note: 49, instrument: ins("Lead saw", 0x08, 0xa8, steps([SAW, 0])) },
  { category: "Lead", note: 49, instrument: ins("Lead square", 0x09, 0xa9, steps([PULSE, 0])) },
  { category: "Lead", note: 49, instrument: ins("Pluck", 0x06, 0x06, steps([PULSE, 0]), { pulse_width: 0x300 }) },
  { category: "Pad", note: 49, instrument: ins("Pad", 0x6a, 0xa9, steps([PULSE, 0]), { pulse_speed: 3, pulse_min: 0x400, pulse_max: 0xc00 }) },
  { category: "Arpeggio", note: 49, instrument: ins("Arp major", 0x08, 0xa8, steps([PULSE, 0]), { pulse_width: 0x400, default_chord: 1 }), chord: { notes: [0, 4, 7], hold: false } },
  { category: "Arpeggio", note: 49, instrument: ins("Arp minor", 0x08, 0xa8, steps([PULSE, 0]), { pulse_width: 0x400, default_chord: 1 }), chord: { notes: [0, 3, 7], hold: false } },
];
