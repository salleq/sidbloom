// Mirrors core/src/model.rs (serde JSON).
export const NOTE_OFF = 0xff;
export const FX_NONE = 0xff;
export const CHANNELS = 3;

export const FX = { ARPEGGIO: 0x0, SLIDE_UP: 0x1, SLIDE_DOWN: 0x2, CHORD: 0x3, VIBRATO: 0x4, FILTER_CUTOFF: 0x8, FILTER_RES_ROUTE: 0x9, FILTER_MODE_VOL: 0xa, FILTER_SWEEP: 0xb, SET_SPEED: 0xf } as const;
export const FX_HELP: Record<number, string> = {
  0x0: "Arpeggio (x,y semitones)",
  0x1: "Slide up (units/tick)",
  0x2: "Slide down (units/tick)",
  0x3: "Chord (number; chord instruments only)",
  0x4: "Vibrato (speed, depth)",
  0x8: "Filter cutoff (00–FF)",
  0x9: "Filter resonance, routing (r,v)",
  0xa: "Filter mode, volume (m,v)",
  0xb: "Filter cutoff sweep (signed/tick)",
  0xf: "Set speed (ticks/row)",
};
export const VALID_FX = Object.keys(FX_HELP).map(Number);

export interface Cell { note: number; inst: number; fx: number; fx_param: number }
export interface Pattern { rows: Cell[] }
export interface WaveStep { wave: number; semis: number }
export interface Instrument {
  name: string;
  attack_decay: number;
  sustain_release: number;
  pulse_width: number;
  wave_table: WaveStep[];
  wave_loop: number | null;
  pulse_speed: number;
  pulse_min: number;
  pulse_max: number;
  default_chord: number;
}
export interface Chord { notes: number[]; hold: boolean }
export interface FilterSettings { cutoff: number; resonance: number; routing: number; mode: number; volume: number }
export interface Song {
  title: string;
  author: string;
  released: string;
  pal: boolean;
  speed: number;
  filter: FilterSettings;
  patterns: Pattern[];
  orders: [number[], number[], number[]];
  instruments: Instrument[];
  chords: Chord[];
  /** Editing aid (see core/src/model.rs): pattern 3n+ch is voice ch of combined pattern n. */
  linked?: boolean;
}

export const LIMITS = { instruments: 64, patterns: 128, rows: 64, waveSteps: 128, chords: 32, chordLen: 16 };

export const emptyCell = (): Cell => ({ note: 0, inst: 0, fx: FX_NONE, fx_param: 0 });
export const emptyPattern = (len = 32): Pattern => ({ rows: Array.from({ length: len }, emptyCell) });

export const WAVE_BITS = [
  { bit: 0x10, label: "Tri" },
  { bit: 0x20, label: "Saw" },
  { bit: 0x40, label: "Pulse" },
  { bit: 0x80, label: "Noise" },
  { bit: 0x04, label: "Ring" },
  { bit: 0x02, label: "Sync" },
] as const;

const NAMES = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];
export function noteName(n: number): string {
  if (n === 0) return "···";
  if (n === NOTE_OFF) return "===";
  const v = n - 1;
  return NAMES[v % 12] + Math.floor(v / 12);
}
export const hex = (v: number, digits = 2) => v.toString(16).toUpperCase().padStart(digits, "0");

export function cellText(c: Cell) {
  return {
    note: noteName(c.note),
    inst: c.inst ? hex(c.inst) : "··",
    fx: c.fx === FX_NONE ? "·" : hex(c.fx, 1),
    param: c.fx === FX_NONE ? "··" : hex(c.fx_param),
  };
}

/** Pattern lookup helpers (order lists may differ in length in imported songs). */
export const orderLength = (s: Song) => Math.max(...s.orders.map((o) => o.length));
export const patternAt = (s: Song, ch: number, pos: number): Pattern | undefined => {
  const idx = s.orders[ch][pos];
  return idx === undefined ? undefined : s.patterns[idx];
};
export const rowsAt = (s: Song, pos: number) => Math.max(1, ...[0, 1, 2].map((ch) => patternAt(s, ch, pos)?.rows.length ?? 0));

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/**
 * A copy of the song whose order list is just position `pos`, so the engine loops that one pattern.
 * A voice without a pattern there gets an empty one, so it stays silent.
 */
export function patternLoop(s: Song, pos: number): Song {
  const rows = rowsAt(s, pos);
  const patterns = [...s.patterns];
  let blank = -1;
  const orders = [0, 1, 2].map((ch) => {
    const idx = s.orders[ch][pos];
    if (idx !== undefined) return [idx];
    if (blank < 0) { blank = patterns.length; patterns.push(emptyPattern(rows)); }
    return [blank];
  }) as Song["orders"];
  return { ...s, patterns, orders };
}

/** Tracker piano: key -> semitone offset from the current octave's C. */
export const PIANO_KEYS: Record<string, number> = {
  z: 0, s: 1, x: 2, d: 3, c: 4, v: 5, g: 6, b: 7, h: 8, n: 9, j: 10, m: 11, ",": 12, l: 13, ".": 14, ";": 15, "/": 16,
  q: 12, "2": 13, w: 14, "3": 15, e: 16, r: 17, "5": 18, t: 19, "6": 20, y: 21, "7": 22, u: 23, i: 24, "9": 25, o: 26, "0": 27, p: 28,
};
export const noteFromKey = (key: string, octave: number): number | null => {
  const off = PIANO_KEYS[key.toLowerCase()];
  if (off === undefined) return null;
  const n = octave * 12 + off + 1;
  return n >= 1 && n <= 95 ? n : null;
};

/** Number of combined patterns in a linked song. */
export const groupCount = (s: Song) => Math.floor(s.patterns.length / 3);
/** Combined pattern number shown at an order position of a linked song. */
export const groupAt = (s: Song, pos: number) => Math.floor((s.orders[0][pos] ?? 0) / 3);

/** Why a song cannot be switched to linked patterns (null: it can). Voices advance independently in the
 *  player, so differing lengths would change the timing. */
export function linkProblem(s: Song): string | null {
  const n = s.orders[0].length;
  if (s.orders.some((o) => o.length !== n)) return "the voices have different numbers of positions";
  for (let p = 0; p < n; p++) {
    const lens = s.orders.map((o) => s.patterns[o[p]]?.rows.length);
    if (lens.some((l) => l === undefined || l !== lens[0])) return `the voices' patterns at position ${p.toString(16).toUpperCase().padStart(2, "0")} have different lengths`;
  }
  return null;
}

/**
 * A copy of the song in which the voices that are not `audible` are silent: each of their patterns is replaced by one
 * of the same length that selects a silent instrument and switches the gate off, so a note that is sounding (even a pad
 * with a long release) stops at once. Positions stay the same.
 */
export function silenced(s: Song, audible: boolean[]): Song {
  if (audible.every(Boolean)) return s;
  const patterns = [...s.patterns];
  const instruments = [...s.instruments, { name: "mute", attack_decay: 0, sustain_release: 0, pulse_width: 0x800, wave_table: [{ wave: 0, semis: 0 }], wave_loop: null, pulse_speed: 0, pulse_min: 0, pulse_max: 0x0fff, default_chord: 0 }];
  const blanks = new Map<number, number>();
  const orders = s.orders.map((order, ch) => {
    if (audible[ch]) return order;
    return order.map((idx) => {
      const len = s.patterns[idx]?.rows.length ?? 1;
      let b = blanks.get(len);
      if (b === undefined) {
        b = patterns.length;
        patterns.push({ rows: Array.from({ length: len }, () => ({ ...emptyCell(), note: NOTE_OFF, inst: instruments.length })) });
        blanks.set(len, b);
      }
      return b;
    });
  }) as Song["orders"];
  return { ...s, patterns, instruments, orders };
}
