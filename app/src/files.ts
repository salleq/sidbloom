import type { Chord, Instrument, Song } from "./model";
import { extractProject } from "./wasm";

const inTauri = () => "__TAURI_INTERNALS__" in window;

/** Saves bytes with a native dialog in the desktop app, or as a browser download. */
export async function saveFile(name: string, bytes: Uint8Array | string, ext: string): Promise<boolean> {
  const data = typeof bytes === "string" ? new TextEncoder().encode(bytes) : bytes;
  if (inTauri()) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({ defaultPath: name, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
    if (!path) return false;
    await writeFile(path, data);
    return true;
  }
  const url = URL.createObjectURL(new Blob([data as BlobPart]));
  const a = Object.assign(document.createElement("a"), { href: url, download: name });
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}

/** Asks before unsaved work is replaced; native dialog in the desktop app. */
export async function confirmDiscard(what: string): Promise<boolean> {
  const msg = `This song has unsaved changes. ${what} will discard them. Continue?`;
  if (inTauri()) {
    const { ask } = await import("@tauri-apps/plugin-dialog");
    return ask(msg, { title: "Unsaved changes", kind: "warning", okLabel: "Discard changes", cancelLabel: "Cancel" });
  }
  return window.confirm(msg);
}

/** Picks a project: a `.sidb`, or a `.sid` / `.prg` that SIDbloom exported (its project is embedded). */
export function openProject(): Promise<{ song: Song; name: string } | null> {
  return new Promise((resolve) => {
    const input = Object.assign(document.createElement("input"), { type: "file", accept: ".sidb,.json,.sid,.prg" });
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      try {
        const bytes = new Uint8Array(await f.arrayBuffer());
        let song: Song;
        if (/\.(sid|prg)$/i.test(f.name)) {
          song = extractProject(bytes);
        } else {
          song = JSON.parse(new TextDecoder().decode(bytes)) as Song;
          if (!song.patterns || !song.orders || !song.instruments) throw new Error("not a SIDbloom project");
          song.chords ??= [];
        }
        resolve({ song, name: f.name.replace(/\.(sid|prg)$/i, ".sidb") });
      } catch (e) {
        alert(`Could not open ${f.name}: ${e instanceof Error ? e.message : e}`);
        resolve(null);
      }
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Picks a MIDI file. */
export function pickMidi(): Promise<{ bytes: Uint8Array; name: string } | null> {
  return pickBytes(".mid,.midi,.rmi");
}

function pickBytes(accept: string): Promise<{ bytes: Uint8Array; name: string } | null> {
  return new Promise((resolve) => {
    const input = Object.assign(document.createElement("input"), { type: "file", accept });
    input.onchange = async () => {
      const f = input.files?.[0];
      resolve(f ? { bytes: new Uint8Array(await f.arrayBuffer()), name: f.name } : null);
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Picks a SID-Wizard module and returns its bytes. */
export function pickSwm(): Promise<{ bytes: Uint8Array; name: string } | null> {
  return new Promise((resolve) => {
    const input = Object.assign(document.createElement("input"), { type: "file", accept: ".swm,.prg,.p00" });
    input.onchange = async () => {
      const f = input.files?.[0];
      resolve(f ? { bytes: new Uint8Array(await f.arrayBuffer()), name: f.name } : null);
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Instrument files (.sidi): one instrument, plus its chord if it plays one. */
export const instrumentFile = (instrument: Instrument, chord?: Chord) =>
  JSON.stringify({ format: "sidbloom-instrument", version: 1, instrument, chord }, null, 2);

export function parseInstrumentFile(text: string): { instrument: Instrument; chord?: Chord } {
  const j = JSON.parse(text);
  const i = j?.instrument;
  const okByte = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) <= 255;
  if (j?.format !== "sidbloom-instrument" || !i || typeof i.name !== "string") throw new Error("not a SIDbloom instrument file");
  if (!okByte(i.attack_decay) || !okByte(i.sustain_release) || !Array.isArray(i.wave_table) || i.wave_table.length < 1 || i.wave_table.length > 128
    || !i.wave_table.every((w: { wave: unknown; semis: unknown }) => okByte(w?.wave) && Number.isInteger(w?.semis) && Math.abs(w.semis as number) <= 96)) {
    throw new Error("the instrument data is damaged");
  }
  const c = j.chord;
  const chord = c && Array.isArray(c.notes) && c.notes.length >= 1 && c.notes.length <= 16 && c.notes.every((n: unknown) => Number.isInteger(n) && Math.abs(n as number) <= 48)
    ? { notes: c.notes as number[], hold: !!c.hold } : undefined;
  const clamp = (v: unknown, lo: number, hi: number, d: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(lo, Math.round(v as number))) : d);
  return {
    chord,
    instrument: {
      name: i.name.slice(0, 24),
      attack_decay: i.attack_decay, sustain_release: i.sustain_release,
      pulse_width: clamp(i.pulse_width, 0, 4095, 0x800),
      wave_table: i.wave_table.map((w: { wave: number; semis: number }) => ({ wave: w.wave, semis: w.semis })),
      wave_loop: Number.isInteger(i.wave_loop) && i.wave_loop >= 0 && i.wave_loop < i.wave_table.length ? i.wave_loop : null,
      pulse_speed: clamp(i.pulse_speed, -64, 64, 0), pulse_min: clamp(i.pulse_min, 0, 4095, 0), pulse_max: clamp(i.pulse_max, 0, 4095, 4095),
      default_chord: chord ? 1 : 0,
    },
  };
}

/** Picks an instrument file. */
export async function pickInstrument(): Promise<{ instrument: Instrument; chord?: Chord } | null> {
  const f = await pickBytes(".sidi,.json");
  if (!f) return null;
  try {
    return parseInstrumentFile(new TextDecoder().decode(f.bytes));
  } catch (e) {
    alert(`Could not open ${f.name}: ${e instanceof Error ? e.message : e}`);
    return null;
  }
}
