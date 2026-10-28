import "./polyfill";
import { initSync, Player } from "./wasm/sidbloom_wasm.js";

declare const sampleRate: number;
declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
  constructor();
}

type In =
  | { t: "init"; bytes: ArrayBuffer }
  | { t: "play"; song: string; is8580: boolean; order: number; row: number }
  | { t: "stop" };

class SidProcessor extends AudioWorkletProcessor {
  private player: Player | null = null;
  private ready = false;
  private frames = 0;

  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<In>) => {
      const m = e.data;
      if (m.t === "init") {
        try {
          initSync({ module: m.bytes });
          this.ready = true;
          this.port.postMessage({ t: "ready" });
        } catch (err) {
          this.port.postMessage({ t: "error", message: `wasm init failed: ${err}` });
        }
      } else if (m.t === "play" && this.ready) {
        this.player?.free();
        try {
          this.player = new Player(m.song, m.is8580, sampleRate, m.order, m.row);
        } catch (err) {
          this.player = null;
          this.port.postMessage({ t: "error", message: String(err) });
        }
      } else if (m.t === "stop") {
        this.player?.free();
        this.player = null;
      }
    };
  }

  process(_in: Float32Array[][], outputs: Float32Array[][]): boolean {
    const out = outputs[0][0];
    if (this.player) {
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
