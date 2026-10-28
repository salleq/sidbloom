import { useId } from "react";
import {
  FX_HELP, LIMITS, WAVE_BITS, hex, orderLength, patternAt, type Instrument, type Song,
} from "../model";
import { edits, type Action } from "../store";

type D = (a: Action) => void;

/* ---------- order list ---------- */
export function OrderPanel({ song, orderPos, ch, dispatch, playOrder }: { song: Song; orderPos: number; ch: number; dispatch: D; playOrder: number | null }) {
  const n = orderLength(song);
  const len = patternAt(song, ch, orderPos)?.rows.length ?? 0;
  return (
    <section className="panel">
      <h2>Song order</h2>
      <div className="order-list" role="listbox" aria-label="Order list">
        {Array.from({ length: n }, (_, p) => (
          <button key={p} role="option" aria-selected={p === orderPos}
            className={`order-row${p === orderPos ? " sel" : ""}${p === playOrder ? " playing" : ""}`}
            onClick={() => dispatch({ t: "orderPos", pos: p })}>
            <b>{hex(p)}</b>
            {[0, 1, 2].map((c) => <span key={c}>{song.orders[c][p] === undefined ? "--" : hex(song.orders[c][p])}</span>)}
          </button>
        ))}
      </div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.insertOrder(orderPos) })} title="Insert a position after this one">+ Position</button>
        <button onClick={() => dispatch({ t: "edit", fn: edits.deleteOrder(orderPos) })} disabled={n <= 1}>− Position</button>
      </div>
      <div className="sub">Voice {ch + 1} at position {hex(orderPos)}</div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.bumpPattern(ch, orderPos, -1) })} aria-label="Previous pattern">◀</button>
        <span className="mono">pattern {hex(song.orders[ch][orderPos] ?? 0)}</span>
        <button onClick={() => dispatch({ t: "edit", fn: edits.bumpPattern(ch, orderPos, 1) })} aria-label="Next pattern">▶</button>
      </div>
      <div className="btn-row">
        <button onClick={() => dispatch({ t: "edit", fn: edits.newPatternFor(ch, orderPos) })} disabled={song.patterns.length >= LIMITS.patterns}>New</button>
        <button onClick={() => dispatch({ t: "edit", fn: edits.duplicatePatternFor(ch, orderPos) })} disabled={song.patterns.length >= LIMITS.patterns}>Duplicate</button>
      </div>
      <label className="field">Rows
        <input type="number" min={1} max={LIMITS.rows} value={len}
          onChange={(e) => dispatch({ t: "edit", fn: edits.setPatternLength(ch, orderPos, Number(e.target.value) || 1) })} />
      </label>
    </section>
  );
}

/* ---------- instrument editor ---------- */
function Slider({ label, value, max, onChange, hexOut = true }: { label: string; value: number; max: number; onChange: (v: number) => void; hexOut?: boolean }) {
  const id = useId();
  return (
    <label className="slider" htmlFor={id}>
      <span>{label}</span>
      <input id={id} type="range" min={0} max={max} value={value} onChange={(e) => onChange(Number(e.target.value))} />
      <output>{hexOut ? hex(value, max > 255 ? 3 : 1) : value}</output>
    </label>
  );
}

// Approximate SID envelope times (ms) for the attack and decay/release rates 0..15.
const ATTACK_MS = [2, 8, 16, 24, 38, 56, 68, 80, 100, 250, 500, 800, 1000, 3000, 5000, 8000];
const DECAY_MS = [6, 24, 48, 72, 114, 168, 204, 240, 300, 750, 1500, 2400, 3000, 9000, 15000, 24000];

function EnvelopeGraph({ a, d, s, r }: { a: number; d: number; s: number; r: number }) {
  const W = 220, H = 70;
  const sus = s / 15;
  const ta = Math.log10(ATTACK_MS[a] + 1), td = Math.log10(DECAY_MS[d] * (1 - sus) + 1), tr = Math.log10(DECAY_MS[r] * Math.max(sus, 0.1) + 1);
  const total = ta + td + 0.6 + tr || 1;
  const x = (t: number) => (t / total) * (W - 8) + 4;
  const y = (v: number) => H - 6 - v * (H - 12);
  const p = [[4, 0], [x(ta), 1], [x(ta + td), sus], [x(ta + td + 0.6), sus], [x(total), 0]];
  return (
    <svg className="env" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Envelope shape">
      <polyline points={p.map(([px, v], i) => `${px},${i === 0 ? y(0) : y(v)}`).join(" ")} />
    </svg>
  );
}

export function InstrumentPanel({ song, inst, dispatch, onPreview }: { song: Song; inst: number; dispatch: D; onPreview: () => void }) {
  const i = inst - 1;
  const ins = song.instruments[i];
  const upd = (fn: (x: Instrument) => void) => dispatch({ t: "edit", fn: edits.updateInstrument(i, fn) });
  if (!ins) return null;
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
        <button onClick={onPreview} title="Hear this instrument">♪ Test</button>
      </div>
      <label className="field">Name
        <input value={ins.name} maxLength={24} onChange={(e) => upd((x) => { x.name = e.target.value; })} />
      </label>
      <EnvelopeGraph a={A} d={D} s={S} r={R} />
      <Slider label="Attack" value={A} max={15} onChange={(v) => upd((x) => { x.attack_decay = (v << 4) | (x.attack_decay & 15); })} />
      <Slider label="Decay" value={D} max={15} onChange={(v) => upd((x) => { x.attack_decay = (x.attack_decay & 0xf0) | v; })} />
      <Slider label="Sustain" value={S} max={15} onChange={(v) => upd((x) => { x.sustain_release = (v << 4) | (x.sustain_release & 15); })} />
      <Slider label="Release" value={R} max={15} onChange={(v) => upd((x) => { x.sustain_release = (x.sustain_release & 0xf0) | v; })} />
      <Slider label="Pulse width" value={ins.pulse_width} max={4095} onChange={(v) => upd((x) => { x.pulse_width = v; })} />
      <h3>Waveform table <small>one step per frame</small></h3>
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
    </section>
  );
}

/* ---------- filter ---------- */
export function FilterPanel({ song, dispatch }: { song: Song; dispatch: D }) {
  const f = song.filter;
  const upd = (fn: (f: Song["filter"]) => void) => dispatch({ t: "edit", fn: (s) => fn(s.filter) });
  return (
    <section className="panel">
      <h2>Filter &amp; volume</h2>
      <Slider label="Cutoff" value={f.cutoff} max={2047} onChange={(v) => upd((x) => { x.cutoff = v; })} />
      <Slider label="Resonance" value={f.resonance} max={15} onChange={(v) => upd((x) => { x.resonance = v; })} />
      <Slider label="Volume" value={f.volume} max={15} onChange={(v) => upd((x) => { x.volume = v; })} />
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
        <dt>[ ]  − =</dt><dd>Octave · instrument</dd>
        <dt>Space / Enter</dt><dd>Play song / play from cursor</dd>
        <dt>Ctrl+Z / Y / S / O</dt><dd>Undo · redo · save · open</dd>
      </dl>
      <h3>Effects</h3>
      <dl>{Object.entries(FX_HELP).map(([k, v]) => (<span key={k} className="fxline"><dt>{Number(k).toString(16).toUpperCase()}xy</dt><dd>{v}</dd></span>))}</dl>
    </section>
  );
}
