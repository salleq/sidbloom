import {
  FX_HELP, LIMITS, WAVE_BITS, groupAt, hex, linkProblem, orderLength, patternAt, type Chord, type Instrument, type Song,
} from "../model";
import { LIBRARY, type Preset } from "../library";
import { instrumentFile, pickInstrument, saveFile } from "../files";
import { useState } from "react";
import { edits, unusedInstruments, unusedPatternCount, type Action, type InstrumentClip, type InstrumentParts } from "../store";
import { EnvelopeEditor, FilterPad, Knob } from "./Controls";

type D = (a: Action) => void;

/* ---------- order list ---------- */
export function OrderPanel({ song, orderPos, ch, dispatch, playOrder, onPlay }: { song: Song; orderPos: number; ch: number; dispatch: D; playOrder: number | null; onPlay: (pos: number) => void }) {
  const n = orderLength(song);
  const len = patternAt(song, ch, orderPos)?.rows.length ?? 0;
  const room = song.patterns.length + 3 <= LIMITS.patterns;
  const unused = unusedPatternCount(song);
  // the new page is inserted after the current position; move onto it
  const addPage = (fn: (s: Song) => void) => {
    dispatch({ t: "edit", fn });
    dispatch({ t: "orderPos", pos: orderPos + 1 });
  };
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);
  // drop on a row: move there (hold Ctrl / Alt to copy instead); `to` may be n, the spot after the last position
  const drop = (to: number, copy: boolean) => {
    if (dragFrom === null) return;
    if (copy) {
      if (!room) return;
      dispatch({ t: "edit", fn: edits.copyOrder(dragFrom, to) });
      dispatch({ t: "orderPos", pos: to });
    } else {
      const dest = Math.min(to, n - 1); // the dragged position takes the place of the one it is dropped on
      dispatch({ t: "edit", fn: edits.moveOrder(dragFrom, dest) });
      dispatch({ t: "orderPos", pos: dest });
    }
    setDragFrom(null); setDragOver(null);
  };
  const over = (to: number) => (e: React.DragEvent) => {
    if (dragFrom === null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = e.ctrlKey || e.altKey ? "copy" : "move";
    setDragOver(to);
  };
  return (
    <section className="panel">
      <h2>Song order</h2>
      <div className="order-list" role="listbox" aria-label="Order list">
        {Array.from({ length: n }, (_, p) => (
          <button key={p} role="option" aria-selected={p === orderPos}
            className={`order-row${p === orderPos ? " sel" : ""}${p === playOrder ? " playing" : ""}${dragOver === p ? " over" : ""}`}
            title="Click to select, double-click to play, drag to move (Ctrl / Alt + drag copies)"
            draggable data-testid={`order-row-${p}`}
            onDragStart={(e) => { setDragFrom(p); e.dataTransfer.effectAllowed = "copyMove"; e.dataTransfer.setData("text/plain", String(p)); }}
            onDragEnd={() => { setDragFrom(null); setDragOver(null); }}
            onDragOver={over(p)} onDrop={(e) => { e.preventDefault(); drop(p, e.ctrlKey || e.altKey); }}
            onClick={() => dispatch({ t: "orderPos", pos: p })} onDoubleClick={() => onPlay(p)}>
            <b>{hex(p)}</b>
            {song.linked
              ? <span className="wide">{song.orders[0][p] === undefined ? "--" : hex(Math.floor(song.orders[0][p] / 3))}</span>
              : [0, 1, 2].map((c) => <span key={c}>{song.orders[c][p] === undefined ? "--" : hex(song.orders[c][p])}</span>)}
          </button>
        ))}
        {dragFrom !== null && (
          <div className={`order-end${dragOver === n ? " over" : ""}`} data-testid="order-end" onDragOver={over(n)} onDrop={(e) => { e.preventDefault(); drop(n, e.ctrlKey || e.altKey); }}>
            drop here to put last
          </div>
        )}
      </div>
      <div className="btn-row">
        <button onClick={() => { dispatch({ t: "edit", fn: edits.copyOrder(orderPos, n) }); dispatch({ t: "orderPos", pos: n }); }} disabled={!room} data-testid="copy-to-end"
          title="Add a copy of this position (with copies of its patterns) at the end of the song">Copy to end</button>
      </div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.insertOrder(orderPos) })} title="Insert a position after this one">+ Position</button>
        <button onClick={() => dispatch({ t: "edit", fn: edits.deleteOrder(orderPos) })} disabled={n <= 1} title="Delete this position, and the patterns only it used">− Position</button>
      </div>
      <label className="chip" title={linkProblem(song) && !song.linked ? `Cannot link: ${linkProblem(song)}` : "One pattern number covers all three voices, like a .mod tracker"}>
        <input type="checkbox" aria-label="Linked patterns" checked={!!song.linked} disabled={!song.linked && linkProblem(song) !== null}
          onChange={(e) => dispatch({ t: "edit", fn: edits.setLinked(e.target.checked) })} />
        <span>Linked patterns</span>
      </label>
      {!song.linked && linkProblem(song) && <div className="sub">Cannot link: {linkProblem(song)}.</div>}
      <div className="sub">{song.linked ? `All voices at position ${hex(orderPos)}` : `Voice ${ch + 1} at position ${hex(orderPos)}`}</div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.bumpPattern(ch, orderPos, -1) })} aria-label="Previous pattern">◀</button>
        <span className="mono">pattern {hex(song.linked ? groupAt(song, orderPos) : song.orders[ch][orderPos] ?? 0)}</span>
        <button onClick={() => dispatch({ t: "edit", fn: edits.bumpPattern(ch, orderPos, 1) })} aria-label="Next pattern">▶</button>
      </div>
      <div className="btn-row">
        <button onClick={() => addPage(edits.newPage(orderPos))} disabled={!room} title="Insert a new position with an empty pattern after this one">New</button>
        <button onClick={() => addPage(edits.duplicatePage(orderPos))} disabled={!room} title="Insert a new position with a copy of this pattern after this one">Duplicate</button>
        <button onClick={() => dispatch({ t: "edit", fn: edits.clearPattern(ch, orderPos) })} title={song.linked ? "Empty this pattern in all voices" : `Empty this pattern in voice ${ch + 1}`}>Clear</button>
      </div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.removeUnusedPatterns() })} disabled={!unused} title="Delete the patterns that are not in the song order">
          Remove unused{unused ? ` (${unused})` : ""}
        </button>
      </div>
      <label className="field">Rows
        <input type="number" min={1} max={LIMITS.rows} value={len}
          onChange={(e) => dispatch({ t: "edit", fn: edits.setPatternLength(ch, orderPos, Number(e.target.value) || 1) })} />
      </label>
    </section>
  );
}

/* ---------- instrument editor ---------- */
export function InstrumentPanel({ song, inst, clip, dispatch, onPreview }: { song: Song; inst: number; clip: InstrumentClip | null; dispatch: D; onPreview: (note?: number) => void }) {
  const i = inst - 1;
  const [parts, setParts] = useState<InstrumentParts>({ adsr: true, wave: false, pulse: false, chord: false });
  const ins = song.instruments[i];
  const upd = (fn: (x: Instrument) => void, key = "") => dispatch({ t: "edit", fn: edits.updateInstrument(i, fn), coalesce: key ? `inst${i}:${key}` : undefined });
  if (!ins) return null;
  const full = song.instruments.length >= LIMITS.instruments;
  const unusedCount = Math.min(unusedInstruments(song).size, song.instruments.length - 1);
  const addFrom = (instrument: Instrument, chord?: Chord, note?: number) => {
    dispatch({ t: "edit", fn: edits.addInstrumentFrom(instrument, chord) });
    dispatch({ t: "curInst", i: song.instruments.length + 1 });
    // the new instrument is only in the song after this event; let the audio engine see it before previewing
    setTimeout(() => onPreview(note), 60);
  };
  const exportInst = async () => {
    const chord = ins.default_chord ? song.chords[ins.default_chord - 1] : undefined;
    await saveFile(`${ins.name.trim().replace(/[^a-z0-9-_]+/gi, "_") || "instrument"}.sidi`, instrumentFile(ins, chord), "sidi");
  };
  const [A, D, S, R] = [ins.attack_decay >> 4, ins.attack_decay & 15, ins.sustain_release >> 4, ins.sustain_release & 15];
  return (
    <section className="panel">
      <h2>Instrument</h2>
      <div className="btn-row">
        <select aria-label="Instrument" value={inst} onChange={(e) => dispatch({ t: "curInst", i: Number(e.target.value) })}>
          {song.instruments.map((x, k) => <option key={k} value={k + 1}>{hex(k + 1)} {x.name}</option>)}
        </select>
        <button onClick={() => { dispatch({ t: "edit", fn: edits.addInstrument() }); dispatch({ t: "curInst", i: song.instruments.length + 1 }); }}
          disabled={song.instruments.length >= LIMITS.instruments} title="New instrument">＋</button>
        <button onClick={() => dispatch({ t: "edit", fn: edits.removeInstrument(i) })} disabled={song.instruments.length <= 1} title="Delete this instrument (rows that used it lose their instrument)" data-testid="delete-instrument">✕</button>
        <button onClick={() => onPreview()} title="Hear this instrument">♪ Test</button>
      </div>
      <div className="btn-row">
        <select aria-label="Instrument library" value="" disabled={full}
          onChange={(e) => { const p = LIBRARY[Number(e.target.value)]; if (p) addFrom(p.instrument, p.chord, p.note); }}>
          <option value="">Add from library…</option>
          {[...new Set(LIBRARY.map((p) => p.category))].map((cat) => (
            <optgroup key={cat} label={cat}>
              {LIBRARY.map((p: Preset, k) => (p.category === cat ? <option key={k} value={k}>{p.instrument.name}</option> : null))}
            </optgroup>
          ))}
        </select>
        <button disabled={full} title="Add an instrument from a .sidi file" data-testid="import-instrument"
          onClick={() => void pickInstrument().then((r) => r && addFrom(r.instrument, r.chord))}>Import</button>
        <button title="Save this instrument as a .sidi file" onClick={() => void exportInst()} data-testid="export-instrument">Export</button>
      </div>
      <div className="btn-row" title="Copy this instrument, then paste only the parts you choose into another one">
        <button onClick={() => dispatch({ t: "copyInstrument", i })} data-testid="copy-instrument">Copy</button>
        {([["adsr", "ADSR"], ["wave", "Waveform"], ["pulse", "Pulse"], ["chord", "Chord"]] as const).map(([k, label]) => (
          <label key={k} className="chip"><input type="checkbox" checked={parts[k]} onChange={() => setParts({ ...parts, [k]: !parts[k] })} /><span>{label}</span></label>
        ))}
        <button disabled={!clip || !Object.values(parts).some(Boolean)} data-testid="paste-instrument"
          title={clip ? `Replace the ticked parts of this instrument with those of ${clip.instrument.name}` : "Copy an instrument first"}
          onClick={() => clip && dispatch({ t: "edit", fn: edits.pasteInstrumentParts(i, clip, parts) })}>Paste{clip ? ` from ${clip.instrument.name.trim() || "copy"}` : ""}</button>
      </div>
      <div className="btn-row">
        <button disabled={!unusedCount} onClick={() => dispatch({ t: "edit", fn: edits.removeUnusedInstruments() })} title="Delete the instruments no pattern uses" data-testid="remove-unused-instruments">
          Remove unused{unusedCount ? ` (${unusedCount})` : ""}
        </button>
      </div>
      <label className="field">Name
        <input value={ins.name} maxLength={24} onChange={(e) => upd((x) => { x.name = e.target.value; })} />
      </label>
      <EnvelopeEditor a={A} d={D} s={S} r={R} onChange={(p) => upd((x) => {
        const ad = ((p.a ?? A) << 4) | (p.d ?? D), sr = ((p.s ?? S) << 4) | (p.r ?? R);
        x.attack_decay = ad; x.sustain_release = sr;
      }, "env")} />
      <div className="knobs">
        <Knob label="Attack" value={A} max={15} onChange={(v) => upd((x) => { x.attack_decay = (v << 4) | (x.attack_decay & 15); }, "a")} />
        <Knob label="Decay" value={D} max={15} onChange={(v) => upd((x) => { x.attack_decay = (x.attack_decay & 0xf0) | v; }, "d")} />
        <Knob label="Sustain" value={S} max={15} def={15} onChange={(v) => upd((x) => { x.sustain_release = (v << 4) | (x.sustain_release & 15); }, "s")} />
        <Knob label="Release" value={R} max={15} onChange={(v) => upd((x) => { x.sustain_release = (x.sustain_release & 0xf0) | v; }, "r")} />
      </div>
      <details className="fold" open>
      <summary>Pulse <small>sweep moves the width every frame</small></summary>
      <div className="knobs">
        <Knob label="Width" value={ins.pulse_width} max={4095} def={2048} onChange={(v) => upd((x) => { x.pulse_width = v; }, "pw")} />
        <Knob label="Sweep" value={ins.pulse_speed} min={-64} max={64} format={(v) => String(v)} onChange={(v) => upd((x) => { x.pulse_speed = v; }, "ps")} />
        <Knob label="Sweep min" value={ins.pulse_min} max={4095} onChange={(v) => upd((x) => { x.pulse_min = Math.min(v, x.pulse_max); }, "pmin")} />
        <Knob label="Sweep max" value={ins.pulse_max} max={4095} def={4095} onChange={(v) => upd((x) => { x.pulse_max = Math.max(v, x.pulse_min); }, "pmax")} />
      </div>
      </details>
      <label className="field">Chord
        <select aria-label="Default chord" value={ins.default_chord} onChange={(e) => upd((x) => { x.default_chord = Number(e.target.value); })}>
          <option value={0}>none</option>
          {song.chords.map((_, k) => <option key={k} value={k + 1}>{hex(k + 1)} — {chordText(song.chords[k].notes)}</option>)}
        </select>
      </label>
      <details className="fold">
      <summary>Waveform table <small>one step per frame</small></summary>
      <div className="wave-table">
        {ins.wave_table.map((st, k) => (
          <div key={k} className={`wave-step${ins.wave_loop === k ? " loop" : ""}`}>
            <span className="mono">{hex(k)}</span>
            {WAVE_BITS.map(({ bit, label }) => (
              <label key={bit} className="chip">
                <input type="checkbox" checked={(st.wave & bit) !== 0} onChange={() => upd((x) => { x.wave_table[k].wave ^= bit; })} />
                <span>{label}</span>
              </label>
            ))}
            <input className="semis" type="number" min={-48} max={48} value={st.semis} aria-label={`Semitones step ${k}`}
              onChange={(e) => upd((x) => { x.wave_table[k].semis = Math.max(-48, Math.min(48, Number(e.target.value) || 0)); })} />
            <button className={`mini${ins.wave_loop === k ? " on" : ""}`} title="Loop back to this step" onClick={() => upd((x) => { x.wave_loop = x.wave_loop === k ? null : k; })}>↺</button>
            <button className="mini" title="Remove step" disabled={ins.wave_table.length <= 1}
              onClick={() => upd((x) => { x.wave_table.splice(k, 1); if (x.wave_loop !== null && x.wave_loop >= x.wave_table.length) x.wave_loop = null; })}>×</button>
          </div>
        ))}
      </div>
      <button disabled={ins.wave_table.length >= LIMITS.waveSteps}
        onClick={() => upd((x) => { x.wave_table.push({ ...x.wave_table[x.wave_table.length - 1] }); })}>＋ Step</button>
      </details>
    </section>
  );
}

const chordText = (notes: number[]) => notes.map((n) => (n > 0 ? `+${n}` : `${n}`)).join(" ");

/* ---------- chords ---------- */
export function ChordPanel({ song, dispatch }: { song: Song; dispatch: D }) {
  const parse = (txt: string) => txt.split(/[\s,]+/).filter(Boolean).map(Number).filter((n) => Number.isFinite(n)).map((n) => Math.max(-48, Math.min(48, Math.round(n)))).slice(0, LIMITS.chordLen);
  return (
    <section className="panel">
      <h2>Chords</h2>
      <div className="sub">Semitone steps played one per frame on instruments with a chord. Select another in a pattern with effect 3xx.</div>
      {song.chords.map((c, i) => (
        <div key={i} className="chord-row">
          <b className="mono">{hex(i + 1)}</b>
          <input aria-label={`Chord ${i + 1} notes`} defaultValue={c.notes.join(" ")} key={c.notes.join(",")}
            onBlur={(e) => { const n = parse(e.target.value); if (n.length) dispatch({ t: "edit", fn: edits.updateChord(i, (x) => { x.notes = n; }) }); else e.target.value = c.notes.join(" "); }}
            onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
          <label className="chip"><input type="checkbox" checked={c.hold} onChange={() => dispatch({ t: "edit", fn: edits.updateChord(i, (x) => { x.hold = !x.hold; }) })} /><span title="Hold the last note instead of looping">hold</span></label>
          <button className="mini" title="Remove chord" onClick={() => dispatch({ t: "edit", fn: edits.removeChord(i) })}>×</button>
        </div>
      ))}
      <button onClick={() => dispatch({ t: "edit", fn: edits.addChord() })} disabled={song.chords.length >= LIMITS.chords}>＋ Chord</button>
    </section>
  );
}

/* ---------- filter ---------- */
export function FilterPanel({ song, dispatch }: { song: Song; dispatch: D }) {
  const f = song.filter;
  const upd = (fn: (f: Song["filter"]) => void, key = "") => dispatch({ t: "edit", fn: (s) => fn(s.filter), coalesce: key ? `filter:${key}` : undefined });
  return (
    <section className="panel">
      <h2>Filter &amp; volume</h2>
      <FilterPad cutoff={f.cutoff} resonance={f.resonance} mode={f.mode} onChange={(p) => upd((x) => { x.cutoff = p.cutoff ?? x.cutoff; x.resonance = p.resonance ?? x.resonance; }, "pad")} />
      <div className="knobs">
        <Knob label="Cutoff" value={f.cutoff} max={2047} def={1024} onChange={(v) => upd((x) => { x.cutoff = v; }, "cut")} />
        <Knob label="Resonance" value={f.resonance} max={15} onChange={(v) => upd((x) => { x.resonance = v; }, "res")} />
        <Knob label="Volume" value={f.volume} max={15} def={15} onChange={(v) => upd((x) => { x.volume = v; }, "vol")} />
      </div>
      <div className="chips">
        {[0, 1, 2].map((v) => (
          <label key={v} className="chip"><input type="checkbox" checked={(f.routing & (1 << v)) !== 0} onChange={() => upd((x) => { x.routing ^= 1 << v; })} /><span>Voice {v + 1}</span></label>
        ))}
      </div>
      <div className="chips">
        {([[0x10, "Low-pass"], [0x20, "Band-pass"], [0x40, "High-pass"]] as const).map(([bit, name]) => (
          <label key={bit} className="chip"><input type="checkbox" checked={(f.mode & bit) !== 0} onChange={() => upd((x) => { x.mode ^= bit; })} /><span>{name}</span></label>
        ))}
      </div>
    </section>
  );
}

export function HelpPanel() {
  return (
    <section className="panel help">
      <h2>Keys</h2>
      <dl>
        <dt>Z S X D C V G B H N J M</dt><dd>Piano, lower octave (Q 2 W 3 E … upper)</dd>
        <dt>1 or `</dt><dd>Note off</dd>
        <dt>0–9, A–F</dt><dd>Enter hex in instrument / effect columns</dd>
        <dt>Arrows, Tab</dt><dd>Move · PgUp/PgDn jump 16 rows</dd>
        <dt>Del / Backspace</dt><dd>Clear cell · Ins inserts a row</dd>
        <dt>&lt; &gt; · F1 F2 · [ ]</dt><dd>Octave down / up</dd>
        <dt>− = · F3 F4</dt><dd>Previous / next instrument</dd>
        <dt>Space</dt><dd>Play the song from the current pattern · Shift+Space from the top · Ctrl+Space loops the current pattern</dd>
        <dt>Enter · Shift+Enter</dt><dd>Play the song from the cursor · from the start of the current pattern (same as Space)</dd>
        <dt>Ctrl+A · Ctrl+[ ]</dt><dd>Select the whole pattern · transpose selection ±1 (Shift: ±12)</dd>
        <dt>Copy / Paste in a voice header</dt><dd>Copy or replace a whole track of the pattern, no selecting needed</dd>
        <dt>Drag, Shift+arrows</dt><dd>Select cells · Ctrl+C / X / V copy, cut, paste</dd>
        <dt>Ctrl+Z / Y / O</dt><dd>Undo · redo · open</dd>
        <dt>Ctrl+S · Ctrl+Shift+S</dt><dd>Save · save as</dd>
      </dl>
      <h3>Effects</h3>
      <dl>{Object.entries(FX_HELP).map(([k, v]) => (<span key={k} className="fxline"><dt>{Number(k).toString(16).toUpperCase()}xy</dt><dd>{v}</dd></span>))}</dl>
    </section>
  );
}
