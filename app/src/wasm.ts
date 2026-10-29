import init, * as w from "./wasm/sidbloom_wasm.js";
import wasmUrl from "./wasm/sidbloom_wasm_bg.wasm?url";
import type { Song } from "./model";

let ready: Promise<unknown> | null = null;
export const loadWasm = () => (ready ??= init({ module_or_path: wasmUrl }));

export const demoSong = (): Song => JSON.parse(w.demo_song());
export const newSong = (): Song => {
  const s: Song = JSON.parse(w.new_song());
  // every voice gets its own pattern, so editing one voice does not change the others
  while (s.patterns.length < 3) s.patterns.push(structuredClone(s.patterns[0]));
  s.orders = [[0], [1], [2]];
  return s;
};
export const validate = (s: Song) => w.validate(JSON.stringify(s));
export const exportSid = (s: Song, is8580: boolean) => w.export_sid(JSON.stringify(s), is8580);
export const exportPrg = (s: Song) => w.export_prg(JSON.stringify(s));
export const importSwm = (bytes: Uint8Array): { song: Song; warnings: string[] } => JSON.parse(w.import_swm(bytes));
export const extractProject = (bytes: Uint8Array): Song => JSON.parse(w.extract_project(bytes));
export const projectFits = (s: Song) => w.project_fits(JSON.stringify(s));

export interface MidiPart { track: number; channel: number; name: string; notes: number; min: number; max: number; drum: boolean }
export const midiParts = (bytes: Uint8Array): { parts: MidiPart[]; default: (number | null)[] } => JSON.parse(w.midi_parts(bytes));
export const importMidi = (bytes: Uint8Array, voices: (number | null)[]): { song: Song; warnings: string[] } =>
  JSON.parse(w.import_midi(bytes, voices[0] ?? -1, voices[1] ?? -1, voices[2] ?? -1));
