import { useReducer } from "react";
import {
  CHANNELS, FX_NONE, LIMITS, NOTE_OFF, VALID_FX, clamp, emptyCell, emptyPattern, orderLength, patternAt, rowsAt,
  groupAt, groupCount, linkProblem,
  type Cell, type Chord, type Instrument, type Song,
} from "./model";

export interface InstrumentClip { instrument: Instrument; chord?: Chord }
/** Which parts of a copied instrument a paste replaces. */
export interface InstrumentParts { adsr: boolean; wave: boolean; pulse: boolean; chord: boolean }

export interface Cursor { ch: number; row: number; field: number } // field 0 note,1-2 inst,3 fx,4-5 param

export interface Sel { ch: number; row: number } // anchor; the selection spans anchor..cursor
export interface Rect { ch0: number; ch1: number; row0: number; row1: number }

export interface State {
  song: Song;
  past: Song[];
  future: Song[];
  cursor: Cursor;
  sel: Sel | null;
  clip: Cell[][] | null; // clip[ch][row], copied cells
  instClip: InstrumentClip | null; // a copied instrument, parts of which can be pasted into another
  orderPos: number;
  octave: number;
  curInst: number; // 1-based
  step: number;
  sid8580: boolean;
  fileName: string;
  dirty: boolean;
  coal?: { key: string; at: number }; // last coalescable edit, see the "edit" action
}

export type Action =
  | { t: "load"; song: Song; fileName?: string }
  | { t: "saved"; fileName: string }
  | { t: "undo" }
  | { t: "redo" }
  | { t: "cursor"; c: Partial<Cursor>; extend?: boolean }
  | { t: "move"; dch?: number; drow?: number; dfield?: number; extend?: boolean }
  | { t: "deselect" }
  | { t: "copy"; cut?: boolean }
  | { t: "paste" }
  | { t: "copyInstrument"; i: number }
  | { t: "copyTrack"; ch: number }
  | { t: "pasteTrack"; ch: number }
  | { t: "selectAll" }
  | { t: "transpose"; d: number }
  | { t: "orderPos"; pos: number }
  | { t: "octave"; d: number }
  | { t: "curInst"; i: number }
  | { t: "step"; n: number }
  | { t: "sid8580"; v: boolean }
  | { t: "enterNote"; note: number }
  | { t: "enterHex"; digit: number }
  | { t: "clearCell"; backspace?: boolean; whole?: boolean }
  | { t: "insertRow"; remove?: boolean }
  | { t: "edit"; fn: (s: Song) => void; coalesce?: string };

export function initState(song: Song): State {
  return { song, past: [], future: [], cursor: { ch: 0, row: 0, field: 0 }, sel: null, clip: null, instClip: null, orderPos: 0, octave: 4, curInst: 1, step: 1, sid8580: true, fileName: "untitled.sidb", dirty: false };
}

const clone = <T,>(v: T): T => structuredClone(v);

function commit(st: State, fn: (s: Song) => void): State {
  const song = clone(st.song);
  fn(song);
  return { ...st, song, past: [...st.past.slice(-199), st.song], future: [], dirty: true };
}

function cellAt(song: Song, c: Cursor, pos: number): Cell | undefined {
  return patternAt(song, c.ch, pos)?.rows[c.row];
}

function clampCursor(st: State): State {
  const rows = rowsAt(st.song, st.orderPos);
  const c = st.cursor;
  return { ...st, cursor: { ch: clamp(c.ch, 0, CHANNELS - 1), row: clamp(c.row, 0, rows - 1), field: clamp(c.field, 0, 5) } };
}

/** The selected cells, or just the cursor cell. */
export function selRect(st: Pick<State, "sel" | "cursor">): Rect {
  const a = st.sel ?? st.cursor, c = st.cursor;
  return { ch0: Math.min(a.ch, c.ch), ch1: Math.max(a.ch, c.ch), row0: Math.min(a.row, c.row), row1: Math.max(a.row, c.row) };
}

function advance(st: State): State {
  const rows = rowsAt(st.song, st.orderPos);
  return { ...st, cursor: { ...st.cursor, row: (st.cursor.row + st.step) % rows } };
}

export function reducer(st: State, a: Action): State {
  switch (a.t) {
    case "load":
      return { ...initState(a.song), fileName: a.fileName ?? "untitled.sidb", sid8580: st.sid8580, octave: st.octave, clip: st.clip, instClip: st.instClip };
    case "saved": return { ...st, fileName: a.fileName, dirty: false };
    case "undo": {
      if (!st.past.length) return st;
      const song = st.past[st.past.length - 1];
      return clampCursor({ ...st, song, past: st.past.slice(0, -1), future: [st.song, ...st.future], dirty: true });
    }
    case "redo": {
      if (!st.future.length) return st;
      const [song, ...future] = st.future;
      return clampCursor({ ...st, song, future, past: [...st.past, st.song], dirty: true });
    }
    case "cursor": {
      const sel = a.extend ? st.sel ?? { ch: st.cursor.ch, row: st.cursor.row } : null;
      return clampCursor({ ...st, sel, cursor: { ...st.cursor, ...a.c } });
    }
    case "deselect": return st.sel ? { ...st, sel: null } : st;
    case "copy": {
      const r = selRect(st);
      const clip = Array.from({ length: r.ch1 - r.ch0 + 1 }, (_, i) =>
        Array.from({ length: r.row1 - r.row0 + 1 }, (_, j) => structuredClone(patternAt(st.song, r.ch0 + i, st.orderPos)?.rows[r.row0 + j] ?? emptyCell())));
      const next = { ...st, clip };
      return a.cut ? { ...reducer(next, { t: "clearCell", whole: true }), clip } : next;
    }
    case "copyInstrument": {
      const instrument = st.song.instruments[a.i];
      if (!instrument) return st;
      const chord = instrument.default_chord ? st.song.chords[instrument.default_chord - 1] : undefined;
      return { ...st, instClip: structuredClone({ instrument, chord }) };
    }
    case "copyTrack": {
      const rows = patternAt(st.song, a.ch, st.orderPos)?.rows;
      return rows ? { ...st, clip: [structuredClone(rows)] } : st;
    }
    case "pasteTrack": {
      const src = st.clip?.[0];
      if (!src || !patternAt(st.song, a.ch, st.orderPos)) return st;
      const next = commit(st, (s) => {
        const rows = patternAt(s, a.ch, st.orderPos)!.rows;
        for (let j = 0; j < Math.min(src.length, rows.length); j++) {
          const c = structuredClone(src[j]);
          if (c.inst > s.instruments.length) c.inst = 0;
          rows[j] = c;
        }
      });
      return { ...next, sel: null };
    }
    case "selectAll": {
      const rows = rowsAt(st.song, st.orderPos);
      return { ...st, sel: { ch: 0, row: 0 }, cursor: { ...st.cursor, ch: CHANNELS - 1, row: rows - 1 } };
    }
    case "transpose": {
      const r = selRect(st);
      return commit(st, (s) => {
        for (let c = r.ch0; c <= r.ch1; c++) {
          const rows = patternAt(s, c, st.orderPos)?.rows;
          for (let i = r.row0; rows && i <= r.row1 && i < rows.length; i++) {
            const cell = rows[i];
            if (cell.note >= 1 && cell.note <= 95) cell.note = clamp(cell.note + a.d, 1, 95);
          }
        }
      });
    }
    case "paste": {
      if (!st.clip) return st;
      const { ch, row } = st.cursor, clip = st.clip;
      const next = commit(st, (s) => {
        clip.forEach((col, i) => col.forEach((cell, j) => {
          const rows = patternAt(s, ch + i, st.orderPos)?.rows;
          if (ch + i >= CHANNELS || !rows || row + j >= rows.length) return;
          const c = structuredClone(cell);
          if (c.inst > s.instruments.length) c.inst = 0;
          rows[row + j] = c;
        }));
      });
      return { ...next, sel: null };
    }
    case "move": {
      let { ch, row, field } = st.cursor;
      const rows = rowsAt(st.song, st.orderPos);
      field += a.dfield ?? 0;
      if (field > 5) { field = 0; ch += 1; }
      if (field < 0) { field = 5; ch -= 1; }
      ch += a.dch ?? 0;
      if (a.dch) field = 0;
      if (ch >= CHANNELS) ch = 0;
      if (ch < 0) ch = CHANNELS - 1;
      row = (row + (a.drow ?? 0) + rows) % rows;
      const sel = a.extend ? st.sel ?? { ch: st.cursor.ch, row: st.cursor.row } : null;
      return { ...st, sel, cursor: { ch, row, field } };
    }
    case "orderPos": {
      const pos = clamp(a.pos, 0, orderLength(st.song) - 1);
      return clampCursor({ ...st, orderPos: pos, sel: null });
    }
    case "octave": return { ...st, octave: clamp(st.octave + a.d, 0, 7) };
    case "curInst": return { ...st, curInst: clamp(a.i, 1, Math.max(1, st.song.instruments.length)) };
    case "step": return { ...st, step: clamp(a.n, 0, 16) };
    case "sid8580": return { ...st, sid8580: a.v };
    case "enterNote": {
      const { ch, row } = st.cursor;
      if (!cellAt(st.song, st.cursor, st.orderPos)) return st;
      const next = commit(st, (s) => {
        const cell = patternAt(s, ch, st.orderPos)!.rows[row];
        cell.note = a.note;
        if (a.note !== NOTE_OFF) cell.inst = st.curInst;
      });
      return advance(next);
    }
    case "enterHex": {
      const { field, row, ch } = st.cursor;
      if (field === 0 || !cellAt(st.song, st.cursor, st.orderPos)) return st;
      const d = a.digit;
      if (field === 3 && !VALID_FX.includes(d)) return st;
      const next = commit(st, (s) => {
        const cell = patternAt(s, ch, st.orderPos)!.rows[row];
        if (field === 1) cell.inst = Math.min((cell.inst & 0x0f) | (d << 4), s.instruments.length);
        else if (field === 2) {
          const v = (cell.inst & 0xf0) | d;
          cell.inst = v > s.instruments.length ? clamp(d, 0, s.instruments.length) : v;
        } else if (field === 3) { cell.fx = d; }
        else if (field === 4) { if (cell.fx === FX_NONE) cell.fx = 0; cell.fx_param = (cell.fx_param & 0x0f) | (d << 4); }
        else if (field === 5) { if (cell.fx === FX_NONE) cell.fx = 0; cell.fx_param = (cell.fx_param & 0xf0) | d; }
      });
      // moving right through the digits is how trackers feel natural
      return field === 2 || field === 5 ? advance({ ...next, cursor: { ...next.cursor, field: field === 2 ? 0 : 3 } }) : { ...next, cursor: { ...next.cursor, field: field + 1 } };
    }
    case "clearCell": {
      if (st.sel || a.whole) { // clear whole cells of the selection
        const r = selRect(st);
        const next = commit(st, (s) => {
          for (let c = r.ch0; c <= r.ch1; c++) {
            const rows = patternAt(s, c, st.orderPos)?.rows;
            for (let i = r.row0; rows && i <= r.row1 && i < rows.length; i++) rows[i] = emptyCell();
          }
        });
        return { ...next, sel: a.whole ? st.sel : null };
      }
      const { field, row, ch } = st.cursor;
      if (!cellAt(st.song, st.cursor, st.orderPos)) return st;
      const next = commit(st, (s) => {
        const cell = patternAt(s, ch, st.orderPos)!.rows[row];
        if (field === 0) { cell.note = 0; cell.inst = 0; }
        else if (field <= 2) cell.inst = 0;
        else Object.assign(cell, { fx: FX_NONE, fx_param: 0 });
      });
      return a.backspace ? { ...next, cursor: { ...next.cursor, row: Math.max(0, row - 1) } } : advance(next);
    }
    case "insertRow": {
      const { row, ch } = st.cursor;
      if (!patternAt(st.song, ch, st.orderPos)) return st;
      return commit(st, (s) => {
        const rows = patternAt(s, ch, st.orderPos)!.rows;
        if (a.remove) { rows.splice(row, 1); rows.push(emptyCell()); }
        else { rows.splice(row, 0, emptyCell()); rows.pop(); }
      });
    }
    case "edit": {
      const now = Date.now();
      // Repeated edits of one control (a knob drag, a hex entry) share a single undo step.
      const merge = !!a.coalesce && st.coal?.key === a.coalesce && now - st.coal.at < 800 && st.past.length > 0;
      const base = commit(st, a.fn);
      const next: State = { ...base, coal: a.coalesce ? { key: a.coalesce, at: now } : undefined, past: merge ? st.past : base.past };
      return clampCursor({ ...next, curInst: clamp(next.curInst, 1, Math.max(1, next.song.instruments.length)), orderPos: clamp(next.orderPos, 0, orderLength(next.song) - 1) });
    }
  }
}

export function useStore(initial: Song) {
  return useReducer(reducer, initial, initState);
}

/** 1-based number of a chord in the song, added when it is not there yet (0: no room). */
function chordIndex(s: Song, chord: Chord): number {
  let i = s.chords.findIndex((c) => c.hold === chord.hold && c.notes.join() === chord.notes.join());
  if (i < 0 && s.chords.length < LIMITS.chords) { s.chords.push(structuredClone(chord)); i = s.chords.length - 1; }
  return i + 1;
}

function insertPage(s: Song, pos: number, make: (p: Song["patterns"][number]) => Song["patterns"][number]) {
  const voices = [0, 1, 2].filter((c) => patternAt(s, c, pos));
  if (!voices.length || s.patterns.length + voices.length > LIMITS.patterns) return;
  for (const c of voices) {
    s.patterns.push(make(patternAt(s, c, pos)!));
    s.orders[c].splice(pos + 1, 0, s.patterns.length - 1);
  }
}

/** Instrument numbers (1-based) that no pattern played in the song order uses. */
export function unusedInstruments(s: Song): Set<number> {
  const unused = new Set(s.instruments.map((_, i) => i + 1));
  for (const idx of new Set(s.orders.flat())) for (const c of s.patterns[idx]?.rows ?? []) unused.delete(c.inst);
  return unused;
}

/** Patterns that no order position refers to (in a linked song a whole combined pattern counts as used). */
export function unusedPatternCount(s: Song): number {
  const used = new Set(s.orders.flat().map((i) => (s.linked ? Math.floor(i / 3) : i)));
  return s.patterns.filter((_, i) => !used.has(s.linked ? Math.floor(i / 3) : i)).length;
}

function dropInstruments(s: Song, drop: Set<number>) {
  const map = (n: number) => (n === 0 || drop.has(n) ? 0 : n - [...drop].filter((d) => d < n).length);
  for (const p of s.patterns) for (const c of p.rows) c.inst = map(c.inst);
  s.instruments = s.instruments.filter((_, i) => !drop.has(i + 1));
}

function compactPatterns(s: Song) {
  const used = new Set(s.orders.flat().map((i) => (s.linked ? Math.floor(i / 3) : i)));
  const remap = new Map<number, number>();
  const patterns: Song["patterns"] = [];
  s.patterns.forEach((p, i) => {
    if (!used.has(s.linked ? Math.floor(i / 3) : i)) return;
    remap.set(i, patterns.length);
    patterns.push(p);
  });
  if (patterns.length === s.patterns.length) return;
  s.patterns = patterns;
  for (let c = 0; c < CHANNELS; c++) s.orders[c] = s.orders[c].map((i) => remap.get(i) ?? 0);
}

// ---- song-level edit helpers (used with {t:"edit"}) ----
export const edits = {
  /** Inserts a new position after `pos` with an empty pattern for every voice (the song order gets a new page). */
  newPage: (pos: number) => (s: Song) => insertPage(s, pos, (p) => emptyPattern(p.rows.length)),
  /** Inserts a new position after `pos` holding copies of that position's patterns. */
  duplicatePage: (pos: number) => (s: Song) => insertPage(s, pos, (p) => structuredClone(p)),
  /** Moves position `from` so that it ends up at index `to` of the song order. */
  moveOrder: (from: number, to: number) => (s: Song) => {
    for (const c of [0, 1, 2]) {
      const o = s.orders[c];
      if (from >= o.length) continue;
      const [x] = o.splice(from, 1);
      o.splice(Math.min(to, o.length), 0, x);
    }
  },
  /** Inserts a copy of position `from` (with copies of its patterns) so that it ends up at index `to`. */
  copyOrder: (from: number, to: number) => (s: Song) => {
    const voices = [0, 1, 2].filter((c) => patternAt(s, c, from));
    if (!voices.length || s.patterns.length + voices.length > LIMITS.patterns) return;
    for (const c of voices) {
      s.patterns.push(structuredClone(patternAt(s, c, from)!));
      s.orders[c].splice(Math.min(to, s.orders[c].length), 0, s.patterns.length - 1);
    }
  },
  /** Empties the patterns at `pos`: all voices when linked, else only voice `ch`. */
  clearPattern: (ch: number, pos: number) => (s: Song) => {
    for (const c of s.linked ? [0, 1, 2] : [ch]) {
      const p = patternAt(s, c, pos);
      if (p) p.rows = p.rows.map(emptyCell);
    }
  },
  /** Drops patterns no position refers to, and renumbers the rest. */
  removeUnusedPatterns: () => (s: Song) => compactPatterns(s),
  /** Deletes an instrument; rows that played it become empty-instrument rows and later numbers shift down. */
  removeInstrument: (i: number) => (s: Song) => {
    if (s.instruments.length <= 1 || !s.instruments[i]) return;
    dropInstruments(s, new Set([i + 1]));
  },
  /** Deletes the instruments no pattern in the song uses (at least one instrument stays). */
  removeUnusedInstruments: () => (s: Song) => {
    const unused = unusedInstruments(s);
    if (unused.size >= s.instruments.length) unused.delete(1);
    if (unused.size) dropInstruments(s, unused);
  },
  bumpPattern: (ch: number, pos: number, d: number) => (s: Song) => {
    if (s.linked) {
      const g = clamp(groupAt(s, pos) + d, 0, groupCount(s) - 1);
      for (let c = 0; c < CHANNELS; c++) s.orders[c][pos] = 3 * g + c;
      return;
    }
    const cur = s.orders[ch][pos];
    if (cur === undefined) return;
    s.orders[ch][pos] = clamp(cur + d, 0, s.patterns.length - 1);
  },
  insertOrder: (pos: number) => (s: Song) => {
    for (let ch = 0; ch < CHANNELS; ch++) s.orders[ch].splice(pos + 1, 0, s.orders[ch][pos] ?? 0);
  },
  deleteOrder: (pos: number) => (s: Song) => {
    if (orderLength(s) <= 1) return;
    for (let ch = 0; ch < CHANNELS; ch++) if (s.orders[ch].length > 1) s.orders[ch].splice(pos, 1);
    compactPatterns(s);
  },
  setPatternLength: (ch: number, pos: number, len: number) => (s: Song) => {
    len = clamp(len, 1, LIMITS.rows);
    for (const c of s.linked ? [0, 1, 2] : [ch]) {
      const p = patternAt(s, c, pos);
      if (!p) continue;
      while (p.rows.length < len) p.rows.push(emptyCell());
      p.rows.length = len;
    }
  },
  /** Turns linked patterns on (one group per distinct set of three patterns, copied) or off; does nothing if that is not possible. */
  setLinked: (on: boolean) => (s: Song) => {
    if (!on) { s.linked = false; return; }
    if (s.linked || linkProblem(s) || s.patterns.length === 0) return;
    const groups = new Map<string, number>();
    const patterns: Song["patterns"] = [];
    const orders: number[][] = [[], [], []];
    for (let p = 0; p < s.orders[0].length; p++) {
      const key = s.orders.map((o) => o[p]).join(",");
      let g = groups.get(key);
      if (g === undefined) {
        g = groups.size;
        groups.set(key, g);
        for (let c = 0; c < CHANNELS; c++) patterns.push(structuredClone(s.patterns[s.orders[c][p]]));
      }
      for (let c = 0; c < CHANNELS; c++) orders[c].push(3 * g + c);
    }
    if (patterns.length > LIMITS.patterns) return;
    s.patterns = patterns;
    s.orders = orders as Song["orders"];
    s.linked = true;
  },
  /** Adds a library or imported instrument, and its chord (an identical existing one is reused). */
  addInstrumentFrom: (inst: Instrument, chord?: Chord) => (s: Song) => {
    if (s.instruments.length >= LIMITS.instruments) return;
    const x = structuredClone(inst);
    x.default_chord = chord ? chordIndex(s, chord) : 0;
    s.instruments.push(x);
  },
  /** Replaces the chosen parts of instrument `i` with those of a copied instrument; the other parts stay. */
  pasteInstrumentParts: (i: number, clip: InstrumentClip, parts: InstrumentParts) => (s: Song) => {
    const x = s.instruments[i], src = clip.instrument;
    if (!x) return;
    if (parts.adsr) { x.attack_decay = src.attack_decay; x.sustain_release = src.sustain_release; }
    if (parts.wave) { x.wave_table = structuredClone(src.wave_table); x.wave_loop = src.wave_loop; }
    if (parts.pulse) { x.pulse_width = src.pulse_width; x.pulse_speed = src.pulse_speed; x.pulse_min = src.pulse_min; x.pulse_max = src.pulse_max; }
    if (parts.chord) x.default_chord = clip.chord ? chordIndex(s, clip.chord) : 0;
  },
  addInstrument: () => (s: Song) => {
    if (s.instruments.length >= LIMITS.instruments) return;
    s.instruments.push({ name: `Instrument ${s.instruments.length + 1}`, attack_decay: 0x09, sustain_release: 0xa9, pulse_width: 0x800, wave_table: [{ wave: 0x40, semis: 0 }], wave_loop: null, pulse_speed: 0, pulse_min: 0, pulse_max: 4095, default_chord: 0 });
  },
  addChord: () => (s: Song) => { if (s.chords.length < LIMITS.chords) s.chords.push({ notes: [0, 4, 7], hold: false }); },
  updateChord: (i: number, fn: (c: Song["chords"][number]) => void) => (s: Song) => { if (s.chords[i]) fn(s.chords[i]); },
  removeChord: (i: number) => (s: Song) => {
    s.chords.splice(i, 1);
    // keep references valid: instruments/patterns pointing at removed or shifted chords
    for (const ins of s.instruments) ins.default_chord = ins.default_chord === i + 1 ? 0 : ins.default_chord > i + 1 ? ins.default_chord - 1 : ins.default_chord;
    for (const p of s.patterns) for (const c of p.rows) if (c.fx === 3) c.fx_param = c.fx_param === i + 1 ? 0 : c.fx_param > i + 1 ? c.fx_param - 1 : c.fx_param;
  },
  updateInstrument: (i: number, fn: (inst: Instrument) => void) => (s: Song) => { if (s.instruments[i]) fn(s.instruments[i]); },
};
