// MIDI keyboard input. The desktop app uses the Rust side (midir) because the system WebViews on macOS and
// Linux lack Web MIDI; in a browser (and tests) the Web MIDI API is used.

export interface MidiNote { on: boolean; note: number; velocity: number }
export type MidiListener = (e: MidiNote) => void;

export interface MidiBackend {
  readonly kind: "tauri" | "web";
  ports(): Promise<string[]>;
  open(name: string, onNote: MidiListener): Promise<void>;
  close(): Promise<void>;
}

/** MIDI note 69 (A4) is tracker note 58, i.e. note = midi - 11; returns null outside the tracker's range. */
export const trackerNote = (midi: number): number | null => {
  const n = midi - 11;
  return n >= 1 && n <= 95 ? n : null;
};

/** Parses a raw MIDI message (note on / note off only). */
export function parseMidi(msg: ArrayLike<number>): MidiNote | null {
  if (msg.length < 3) return null;
  const [status, note, velocity] = [msg[0], msg[1], msg[2]];
  if (note > 127 || velocity > 127) return null;
  switch (status & 0xf0) {
    case 0x90: return { on: velocity > 0, note, velocity };
    case 0x80: return { on: false, note, velocity };
    default: return null;
  }
}

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function tauriBackend(): MidiBackend {
  let unlisten: (() => void) | null = null;
  return {
    kind: "tauri",
    async ports() {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<string[]>("midi_ports");
    },
    async open(name, onNote) {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");
      unlisten?.();
      unlisten = await listen<{ on: boolean; note: number; velocity: number }>("midi", (e) => onNote(e.payload));
      await invoke("midi_open", { name });
    },
    async close() {
      unlisten?.();
      unlisten = null;
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("midi_close");
    },
  };
}

function webBackend(): MidiBackend | null {
  if (typeof navigator === "undefined" || !("requestMIDIAccess" in navigator)) return null;
  let access: MIDIAccess | null = null;
  let current: MIDIInput | null = null;
  const get = async () => (access ??= await navigator.requestMIDIAccess());
  return {
    kind: "web",
    async ports() {
      return [...(await get()).inputs.values()].map((i) => i.name ?? i.id);
    },
    async open(name, onNote) {
      const a = await get();
      if (current) current.onmidimessage = null;
      const input = [...a.inputs.values()].find((i) => (i.name ?? i.id) === name);
      if (!input) throw new Error(`MIDI input "${name}" not found`);
      input.onmidimessage = (m) => {
        const ev = m.data ? parseMidi(m.data) : null;
        if (ev) onNote(ev);
      };
      current = input;
    },
    async close() {
      if (current) current.onmidimessage = null;
      current = null;
    },
  };
}

/** The MIDI backend for this environment, or null when MIDI input is unavailable. */
export const createMidiBackend = (): MidiBackend | null => (inTauri() ? tauriBackend() : webBackend());
