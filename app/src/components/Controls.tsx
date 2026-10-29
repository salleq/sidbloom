import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { hex } from "../model";

const clampN = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

/* ---------- knob ---------- */
const SWEEP = 270, START = 135; // degrees, clockwise from 3 o'clock
const pt = (r: number, deg: number): [number, number] => [24 + r * Math.cos((deg * Math.PI) / 180), 24 + r * Math.sin((deg * Math.PI) / 180)];
const arc = (r: number, a0: number, a1: number) => {
  const [x0, y0] = pt(r, a0), [x1, y1] = pt(r, a1);
  return `M${x0},${y0} A${r},${r} 0 ${a1 - a0 > 180 ? 1 : 0} 1 ${x1},${y1}`;
};

interface KnobProps {
  label: string; value: number; max: number; min?: number; def?: number;
  onChange: (v: number) => void; format?: (v: number) => string;
}

/** Rotary control: drag vertically (Shift = fine), wheel, arrow keys, double-click resets. */
export function Knob({ label, value, max, min = 0, def, onChange, format }: KnobProps) {
  const drag = useRef<{ y: number; v: number } | null>(null);
  const [active, setActive] = useState(false);
  const span = max - min;
  const reset = def ?? (min < 0 ? 0 : min);
  const bipolar = min < 0;
  const frac = (value - min) / span;
  const text = format ? format(value) : hex(value, max > 255 ? 3 : 1);
  const set = (v: number) => { const n = clampN(Math.round(v), min, max); if (n !== value) onChange(n); };
  const down = (e: PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { y: e.clientY, v: value };
    setActive(true);
  };
  const move = (e: PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    set(drag.current.v + ((drag.current.y - e.clientY) / (e.shiftKey ? 600 : 150)) * span);
  };
  const up = () => { drag.current = null; setActive(false); };
  const key = (e: KeyboardEvent) => {
    const big = Math.max(1, Math.round(span / 16));
    const d = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: big, PageDown: -big }[e.key];
    if (d !== undefined) { e.preventDefault(); set(value + d); }
    else if (e.key === "Home") { e.preventDefault(); set(min); }
    else if (e.key === "End") { e.preventDefault(); set(max); }
  };
  const a0 = bipolar ? START + SWEEP / 2 : START, a1 = START + frac * SWEEP;
  const lo = Math.min(a0, a1), hi = Math.max(a0, a1);
  const [nx, ny] = pt(13, a1), [ix, iy] = pt(6, a1);
  return (
    <div className={`knob${active ? " active" : ""}`}>
      <div role="slider" tabIndex={0} aria-label={label} aria-valuemin={min} aria-valuemax={max} aria-valuenow={value} aria-valuetext={text}
        onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up}
        onDoubleClick={() => set(reset)} onKeyDown={key}
        onWheel={(e) => { e.preventDefault(); set(value + (e.deltaY < 0 ? 1 : -1) * Math.max(1, Math.round(span / (e.shiftKey ? 256 : 32)))); }}>
        <svg viewBox="0 0 48 48" width="48" height="48" aria-hidden="true">
          <path className="knob-track" d={arc(19, START, START + SWEEP)} />
          {hi - lo > 0.5 && <path className="knob-value" d={arc(19, lo, hi)} />}
          <circle className="knob-cap" cx="24" cy="24" r="13" />
          <line className="knob-tick" x1={ix} y1={iy} x2={nx} y2={ny} />
        </svg>
      </div>
      <output className="knob-val">{text}</output>
      <span className="knob-label">{label}</span>
    </div>
  );
}

/* ---------- envelope editor ---------- */
// Approximate SID envelope times (ms) for the attack and decay/release rates 0..15.
export const ATTACK_MS = [2, 8, 16, 24, 38, 56, 68, 80, 100, 250, 500, 800, 1000, 3000, 5000, 8000];
export const DECAY_MS = [6, 24, 48, 72, 114, 168, 204, 240, 300, 750, 1500, 2400, 3000, 9000, 15000, 24000];

const EW = 240, EH = 84, PAD = 8;
const ZA = 70, ZD = 70, HOLD = 30, ZR = 60; // fixed horizontal zones keep the handles from jumping while dragging
const lg = (ms: number, top: number) => Math.log10(ms + 1) / Math.log10(top + 1);
const nearest = (table: number[], frac: number, top: number) => {
  let best = 0, bd = Infinity;
  table.forEach((ms, i) => { const d = Math.abs(lg(ms, top) - frac); if (d < bd) { bd = d; best = i; } });
  return best;
};

interface EnvProps { a: number; d: number; s: number; r: number; onChange: (p: { a?: number; d?: number; s?: number; r?: number }) => void }

/** ADSR drawn as a shape; drag the three handles (attack peak, decay/sustain corner, release end). */
export function EnvelopeEditor({ a, d, s, r, onChange }: EnvProps) {
  const svg = useRef<SVGSVGElement>(null);
  const [grab, setGrab] = useState<"a" | "d" | "r" | null>(null);
  const y = (v: number) => EH - PAD - v * (EH - 2 * PAD);
  const xA = PAD + ZA * lg(ATTACK_MS[a], 8000);
  const xD = PAD + ZA + ZD * lg(DECAY_MS[d], 24000);
  const xH = PAD + ZA + ZD + HOLD;
  const xR = xH + ZR * lg(DECAY_MS[r], 24000);
  const ys = y(s / 15);
  const at = (e: PointerEvent) => {
    const b = svg.current!.getBoundingClientRect();
    return { x: ((e.clientX - b.left) / b.width) * EW, y: ((e.clientY - b.top) / b.height) * EH };
  };
  const move = (e: PointerEvent) => {
    if (!grab) return;
    const p = at(e);
    if (grab === "a") onChange({ a: nearest(ATTACK_MS, clampN((p.x - PAD) / ZA, 0, 1), 8000) });
    else if (grab === "d") onChange({ d: nearest(DECAY_MS, clampN((p.x - PAD - ZA) / ZD, 0, 1), 24000), s: clampN(Math.round(((EH - PAD - p.y) / (EH - 2 * PAD)) * 15), 0, 15) });
    else onChange({ r: nearest(DECAY_MS, clampN((p.x - xH) / ZR, 0, 1), 24000) });
  };
  const handle = (k: "a" | "d" | "r", cx: number, cy: number, label: string) => (
    <circle className={`env-handle${grab === k ? " on" : ""}`} cx={cx} cy={cy} r={6} role="presentation" aria-label={label}
      onPointerDown={(e) => { e.currentTarget.setPointerCapture(e.pointerId); setGrab(k); }} />
  );
  return (
    <svg ref={svg} className="env env-edit" viewBox={`0 0 ${EW} ${EH}`} role="img" aria-label="Envelope shape: drag the handles"
      onPointerMove={move} onPointerUp={() => setGrab(null)} onPointerCancel={() => setGrab(null)}>
      <line className="env-guide" x1={PAD} x2={EW - PAD} y1={ys} y2={ys} />
      <polyline points={`${PAD},${y(0)} ${xA},${y(1)} ${xD},${ys} ${xH},${ys} ${xR},${y(0)}`} />
      <path className="env-fill" d={`M${PAD},${y(0)} L${xA},${y(1)} L${xD},${ys} L${xH},${ys} L${xR},${y(0)} Z`} />
      {handle("a", xA, y(1), "Attack")}
      {handle("d", xD, ys, "Decay and sustain")}
      {handle("r", xR, y(0), "Release")}
    </svg>
  );
}

/* ---------- filter pad ---------- */
const PW = 240, PH = 110;

interface PadProps { cutoff: number; resonance: number; mode: number; onChange: (p: { cutoff?: number; resonance?: number }) => void }

/** X = cutoff, Y = resonance, over the frequency response of the selected filter modes. */
export function FilterPad({ cutoff, resonance, mode, onChange }: PadProps) {
  const el = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState(false);
  const q = 0.6 + resonance * 0.45;
  const c = (cutoff / 2047) * 0.9 + 0.05; // keep the response curve on screen at the extremes
  const pts: string[] = [];
  for (let i = 0; i <= 96; i++) {
    const w = (i / 96 * 0.95 + 0.05) / c; // frequency relative to the cutoff
    const re = 1 - w * w, im = w / q;
    const den = Math.hypot(re, im);
    let h = 0;
    if (mode & 0x10) h += 1 / den;
    if (mode & 0x20) h += (w / q) / den;
    if (mode & 0x40) h += (w * w) / den;
    if (!(mode & 0x70)) h = 0;
    const db = clampN(20 * Math.log10(Math.max(h, 1e-3)), -30, 24);
    pts.push(`${(i / 96) * PW},${PH - ((db + 30) / 54) * (PH - 8) - 4}`);
  }
  const set = (e: PointerEvent) => {
    const b = el.current!.getBoundingClientRect();
    onChange({
      cutoff: Math.round(clampN((e.clientX - b.left) / b.width, 0, 1) * 2047),
      resonance: Math.round(clampN(1 - (e.clientY - b.top) / b.height, 0, 1) * 15),
    });
  };
  const key = (e: KeyboardEvent) => {
    const k = { ArrowLeft: [-32, 0], ArrowRight: [32, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }[e.key];
    if (!k) return;
    e.preventDefault();
    onChange({ cutoff: clampN(cutoff + k[0], 0, 2047), resonance: clampN(resonance + k[1], 0, 15) });
  };
  const px = (cutoff / 2047) * PW, py = (1 - resonance / 15) * PH;
  return (
    <svg ref={el} className="pad" viewBox={`0 0 ${PW} ${PH}`} tabIndex={0} role="application"
      aria-label={`Filter pad: cutoff ${hex(cutoff, 3)}, resonance ${hex(resonance)}. Arrow keys adjust.`}
      onKeyDown={key}
      onPointerDown={(e) => { e.currentTarget.setPointerCapture(e.pointerId); setDrag(true); set(e); }}
      onPointerMove={(e) => drag && set(e)} onPointerUp={() => setDrag(false)} onPointerCancel={() => setDrag(false)}>
      {[0.25, 0.5, 0.75].map((f) => <line key={f} className="pad-grid" x1={f * PW} x2={f * PW} y1={0} y2={PH} />)}
      <polyline className="pad-curve" points={pts.join(" ")} />
      <line className="pad-guide" x1={px} x2={px} y1={0} y2={PH} />
      <line className="pad-guide" x1={0} x2={PW} y1={py} y2={py} />
      <circle className="pad-dot" cx={px} cy={py} r={7} />
    </svg>
  );
}
