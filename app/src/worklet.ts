import "./polyfill";
import { initSync, new_song, Player } from "./wasm/sidbloom_wasm.js";

declare const sampleRate: number;
declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
  constructor();
}

type In =
  | { t: "init"; bytes: ArrayBuffer }
  | { t: "play"; song: string; is8580: boolean; order: number; row: number }
  | { t: "prepare"; is8580: boolean; pal: boolean }
  | { t: "update"; song: string }
  | { t: "stop" };

class SidProcessor extends AudioWorkletProcessor {
  // One player for the whole session: building the SID chip is slow, and doing it on every Play stalls the audio
  // thread, which then races to catch up (the first notes come out too fast).
  private player: Player | null = null;
  private active = false;
  private ready = false;
  private frames = 0;

  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<In>) => {
      const m = e.data;
      if (m.t === "init") {
        try {
          initSync({ module: m.bytes });
          this.player = new Player(new_song(), true, sampleRate, 0, 0);
          this.ready = true;
          this.port.postMessage({ t: "ready" });
        } catch (err) {
          this.port.postMessage({ t: "error", message: `wasm init failed: ${err}` });
        }
      } else if (m.t === "play" && this.ready) {
        try {
          this.player!.restart(m.song, m.is8580, m.order, m.row);
          this.active = true;
        } catch (err) {
          this.active = false;
          this.port.postMessage({ t: "error", message: String(err) });
        }
      } else if (m.t === "prepare" && this.player) {
        this.player.prepare(m.is8580, m.pal);
      } else if (m.t === "update" && this.player && this.active) {
        try {
          this.player.update(m.song);
        } catch (err) {
          this.port.postMessage({ t: "error", message: String(err) });
        }
      } else if (m.t === "stop") {
        this.active = false;
      }
    };
  }

  process(_in: Float32Array[][], outputs: Float32Array[][]): boolean {
    const out = outputs[0][0];
    if (this.player && this.active) {
      this.player.fill(out);
      for (let c = 1; c < outputs[0].length; c++) outputs[0][c].set(out);
      this.frames += out.length;
      if (this.frames >= sampleRate / 30) {
        this.frames = 0;
        this.port.postMessage({ t: "pos", p: Array.from(this.player.positions()) });
      }
    }
    return true;
  }
}

registerProcessor("sid-processor", SidProcessor);
