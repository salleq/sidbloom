import workletUrl from "./worklet.ts?worker&url";
import wasmUrl from "./wasm/sidbloom_wasm_bg.wasm?url";
import { NOTE_OFF, type Song } from "./model";

export type Positions = [number, number][];

/** Owns the AudioContext + AudioWorklet that runs the Rust engine and reSID. */
export class AudioEngine {
  private ctx: AudioContext | null = null;
  private node: AudioWorkletNode | null = null;
  readonly analyser: { current: AnalyserNode | null } = { current: null };
  private starting: Promise<void> | null = null;
  onPositions: (p: Positions) => void = () => {};
  onError: (m: string) => void = () => {};
  private previewTimer: number | undefined;
  /** Position messages are only forwarded while the whole song plays, not for note previews. */
  private songActive = false;
  /** For tests: number of position messages received. */
  messages = 0;

  private async ensure() {
    if (this.node) return;
    if (!this.starting) {
      this.starting = (async () => {
        const ctx = new AudioContext({ latencyHint: "interactive" });
        await ctx.audioWorklet.addModule(workletUrl);
        const bytes = await (await fetch(wasmUrl)).arrayBuffer(); // a compiled Module cannot be posted to a worklet
        const node = new AudioWorkletNode(ctx, "sid-processor", { numberOfInputs: 0, outputChannelCount: [2] });
        node.onprocessorerror = () => this.onError("audio processor crashed");
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 2048;
        node.connect(analyser);
        analyser.connect(ctx.destination);
        await new Promise<void>((resolve) => {
          node.port.onmessage = (e) => {
            if (e.data.t === "ready") resolve();
            else if (e.data.t === "error") { console.error("audio:", e.data.message); this.onError(e.data.message); }
          };
          node.port.postMessage({ t: "init", bytes }, [bytes]);
        });
        node.port.onmessage = (e) => {
          if (e.data.t === "pos") {
            this.messages++;
            if (!this.songActive) return;
            this.onPositions((e.data.p as number[]).reduce<Positions>((acc, v, i) => (i % 2 ? (acc[acc.length - 1][1] = v, acc) : (acc.push([v, 0]), acc)), []));
          } else if (e.data.t === "error") this.onError(e.data.message);
        };
        this.ctx = ctx;
        this.node = node;
        this.analyser.current = analyser;
      })();
    }
    await this.starting;
    if (this.ctx!.state === "suspended") await this.ctx!.resume();
  }

  async play(song: Song, is8580: boolean, order = 0, row = 0, isPreview = false) {
    await this.ensure();
    this.songActive = !isPreview;
    clearTimeout(this.previewTimer);
    this.node!.port.postMessage({ t: "play", song: JSON.stringify(song), is8580, order, row });
  }

  /** Sends an edited song to the running player; playback continues where it was. */
  update(song: Song) {
    this.node?.port.postMessage({ t: "update", song: JSON.stringify(song) });
  }

  stop() {
    this.songActive = false;
    clearTimeout(this.previewTimer);
    this.node?.port.postMessage({ t: "stop" });
  }

  /** Plays one note with the given 1-based instrument on channel 1. */
  async preview(song: Song, inst: number, note: number, is8580: boolean) {
    const s: Song = structuredClone(song);
    const blank = () => ({ rows: Array.from({ length: 64 }, () => ({ note: 0, inst: 0, fx: 0xff, fx_param: 0 })) });
    s.patterns = [blank(), blank()];
    s.patterns[0].rows[0] = { note, inst, fx: 0xff, fx_param: 0 };
    s.patterns[0].rows[36] = { note: NOTE_OFF, inst: 0, fx: 0xff, fx_param: 0 };
    s.orders = [[0], [1], [1]];
    await this.play(s, is8580, 0, 0, true);
    this.previewTimer = window.setTimeout(() => this.stop(), 1800);
  }

  get running() { return this.ctx?.state === "running"; }
}
