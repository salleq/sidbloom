import init, * as w from "./wasm/sidbloom_wasm.js";
import wasmUrl from "./wasm/sidbloom_wasm_bg.wasm?url";
import type { Song } from "./model";

let ready: Promise<unknown> | null = null;
export const loadWasm = () => (ready ??= init({ module_or_path: wasmUrl }));

export const demoSong = (): Song => JSON.parse(w.demo_song());
export const newSong = (): Song => JSON.parse(w.new_song());
export const validate = (s: Song) => w.validate(JSON.stringify(s));
export const exportSid = (s: Song, is8580: boolean) => w.export_sid(JSON.stringify(s), is8580);
export const exportPrg = (s: Song) => w.export_prg(JSON.stringify(s));
