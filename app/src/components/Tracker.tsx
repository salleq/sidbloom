import { memo, useEffect, useRef } from "react";
import { cellText, groupAt, hex, patternAt, rowsAt, type Song } from "../model";
import type { Cursor, Rect } from "../store";

interface Props {
  song: Song;
  orderPos: number;
  cursor: Cursor;
  sel: Rect | null; // selected cells (null: only the cursor)
  playRows: (number | null)[]; // per channel: playing row in this order position, or null
  onCursor: (c: Partial<Cursor>, extend?: boolean) => void;
  audible: boolean[];
  mute: boolean[];
  solo: boolean[];
  onMute: (ch: number) => void;
  onSolo: (ch: number) => void;
  canPaste: boolean;
  onCopyTrack: (ch: number) => void;
  onPasteTrack: (ch: number) => void;
}

const FIELD_CLASS = ["note", "inst", "inst", "fx", "param", "param"] as const;

export const Tracker = memo(function Tracker({ song, orderPos, cursor, sel, playRows, onCursor, audible, mute, solo, onMute, onSolo, canPaste, onCopyTrack, onPasteTrack }: Props) {
  const rows = rowsAt(song, orderPos);
  const body = useRef<HTMLDivElement>(null);
  const follow = playRows.find((r) => r !== null) ?? null;
  useEffect(() => {
    // keep the playhead (while playing) or the edit cursor in view
    const el = body.current?.querySelector<HTMLElement>(follow !== null ? ".tcell.playing" : ".f.cursor");
    el?.scrollIntoView({ block: "nearest" });
  }, [cursor.row, cursor.ch, orderPos, follow]);
  return (
    <div className="tracker" role="grid" aria-label="Pattern editor" data-testid="tracker">
      <div className="tracker-head">
        <div className="rownum" />
        {[0, 1, 2].map((ch) => (
          <div key={ch} className={`chan-head${audible[ch] ? "" : " silent"}`}>
            <span>Voice {ch + 1} <span className="pat-id">pattern {hex(song.linked ? groupAt(song, orderPos) : song.orders[ch][orderPos] ?? 0)}</span></span>
            <span className="track-btns">
              <button className={`mini${mute[ch] ? " on" : ""}`} aria-pressed={mute[ch]} onClick={() => onMute(ch)} title={`Mute voice ${ch + 1} while playing`} data-testid={`mute-${ch}`}>M</button>
              <button className={`mini${solo[ch] ? " on" : ""}`} aria-pressed={solo[ch]} onClick={() => onSolo(ch)} title={`Solo voice ${ch + 1}: only soloed voices play`} data-testid={`solo-${ch}`}>S</button>
              <button className="mini" onClick={() => onCopyTrack(ch)} title={`Copy all of voice ${ch + 1} in this pattern`} data-testid={`copy-track-${ch}`}>Copy</button>
              <button className="mini" onClick={() => onPasteTrack(ch)} disabled={!canPaste} title={`Replace voice ${ch + 1} in this pattern with the copied track`} data-testid={`paste-track-${ch}`}>Paste</button>
            </span>
          </div>
        ))}
      </div>
      <div className="tracker-body" ref={body}>
        {Array.from({ length: rows }, (_, r) => (
          <div key={r} className={`trow${r % 4 === 0 ? " beat" : ""}${r % 16 === 0 ? " bar" : ""}`}>
            <div className="rownum">{hex(r)}</div>
            {[0, 1, 2].map((ch) => {
              const cell = patternAt(song, ch, orderPos)?.rows[r];
              const playing = playRows[ch] === r;
              const selected = !!sel && ch >= sel.ch0 && ch <= sel.ch1 && r >= sel.row0 && r <= sel.row1;
              if (!cell) return <div key={ch} className="tcell off" />;
              const t = cellText(cell);
              const texts = [t.note, t.inst[0], t.inst[1], t.fx, t.param[0], t.param[1]];
              return (
                <div key={ch} className={`tcell${audible[ch] ? "" : " silent"}${playing ? " playing" : ""}${selected ? " selected" : ""}${cell.note ? " has-note" : ""}`}>
                  {texts.map((txt, f) => {
                    const active = cursor.ch === ch && cursor.row === r && cursor.field === f;
                    return (
                      <span
                        key={f}
                        data-testid={active ? "cursor" : undefined}
                        className={`f ${FIELD_CLASS[f]}${active ? " cursor" : ""}${txt.includes("·") ? " empty" : ""}`}
                        onMouseDown={(e) => { if (e.button === 0) onCursor({ ch, row: r, field: f }, e.shiftKey); }}
                        onMouseEnter={(e) => { if (e.buttons & 1) onCursor({ ch, row: r }, true); }}
                      >
                        {txt}
                      </span>
                    );
                  })}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
});
