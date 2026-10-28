import { useEffect, useRef, useState } from "react";
import type { MidiPart } from "../wasm";

const NOTE = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const keyName = (k: number) => `${NOTE[k % 12]}${Math.floor(k / 12) - 1}`;

interface Props {
  fileName: string;
  parts: MidiPart[];
  initial: (number | null)[];
  onImport: (voices: (number | null)[]) => void;
  onCancel: () => void;
}

/** Chooses which MIDI part plays on which of the three SID voices. */
export function MidiDialog({ fileName, parts, initial, onImport, onCancel }: Props) {
  const [voices, setVoices] = useState<(number | null)[]>([initial[0] ?? null, initial[1] ?? null, initial[2] ?? null]);
  const first = useRef<HTMLSelectElement>(null);
  useEffect(() => {
    first.current?.focus();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);
  const none = voices.every((v) => v === null);

  return (
    <div className="modal-back" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="modal" role="dialog" aria-modal="true" aria-label="Import MIDI file" data-testid="midi-dialog">
        <h2>Import MIDI — {fileName}</h2>
        <p className="sub">
          A SID has three voices that play one note at a time. Choose a part for each voice; when a part plays chords,
          the highest note is kept. The drum channel becomes a noise instrument.
        </p>
        {[0, 1, 2].map((v) => (
          <label key={v} className="field">
            Voice {v + 1}
            <select
              ref={v === 0 ? first : undefined}
              value={voices[v] ?? ""}
              onChange={(e) => setVoices(voices.map((x, i) => (i === v ? (e.target.value === "" ? null : Number(e.target.value)) : x)))}
            >
              <option value="">— silent —</option>
              {parts.map((p, i) => (
                <option key={i} value={i}>
                  {p.name}{p.drum ? " (drums)" : ""} · {p.notes} notes · {keyName(p.min)}–{keyName(p.max)}
                </option>
              ))}
            </select>
          </label>
        ))}
        <div className="btn-row end">
          <button onClick={onCancel}>Cancel</button>
          <button className="primary" disabled={none} onClick={() => onImport(voices)} data-testid="midi-import-go">Import</button>
        </div>
      </div>
    </div>
  );
}
