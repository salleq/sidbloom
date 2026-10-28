import { useEffect, useRef, type ReactElement } from "react";
import { noteName } from "../model";

export function Scope({ analyser }: { analyser: { current: AnalyserNode | null } }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let raf = 0;
    const draw = () => {
      raf = requestAnimationFrame(draw);
      const c = ref.current;
      if (!c) return;
      const g = c.getContext("2d")!;
      const css = getComputedStyle(c);
      g.clearRect(0, 0, c.width, c.height);
      g.strokeStyle = css.getPropertyValue("--accent").trim() || "#7c5cff";
      g.lineWidth = 2;
      g.beginPath();
      const a = analyser.current;
      if (!a) { g.moveTo(0, c.height / 2); g.lineTo(c.width, c.height / 2); g.stroke(); return; }
      const buf = new Float32Array(a.fftSize);
      a.getFloatTimeDomainData(buf);
      let start = 0; // trigger on rising zero crossing for a steady picture
      for (let i = 1; i < buf.length / 2; i++) if (buf[i - 1] < 0 && buf[i] >= 0) { start = i; break; }
      const n = 512;
      for (let i = 0; i < n; i++) {
        const x = (i / (n - 1)) * c.width, y = c.height / 2 - (buf[start + i] ?? 0) * c.height * 0.45;
        i ? g.lineTo(x, y) : g.moveTo(x, y);
      }
      g.stroke();
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, [analyser]);
  return <canvas ref={ref} className="scope" width={480} height={64} aria-label="Oscilloscope" data-testid="scope" />;
}

const WHITE = [0, 2, 4, 5, 7, 9, 11];
const BLACK: Record<number, number> = { 0: 1, 1: 3, 3: 6, 4: 8, 5: 10 }; // white index -> black semitone after it

export function Piano({ octave, onNote }: { octave: number; onNote: (note: number) => void }) {
  const keys: ReactElement[] = [];
  for (let o = 0; o < 2; o++) {
    WHITE.forEach((semi, wi) => {
      const note = (octave + o) * 12 + semi + 1;
      keys.push(
        <button key={`w${o}${wi}`} className="key white" aria-label={noteName(note)} onMouseDown={() => note <= 95 && onNote(note)}>
          {semi === 0 && <small>C{octave + o}</small>}
        </button>,
      );
    });
  }
  const blacks: ReactElement[] = [];
  for (let o = 0; o < 2; o++) {
    Object.entries(BLACK).forEach(([wi, semi]) => {
      const note = (octave + o) * 12 + semi + 1;
      blacks.push(
        <button key={`b${o}${wi}`} className="key black" aria-label={noteName(note)}
          style={{ left: `calc(${(o * 7 + Number(wi) + 1) / 14 * 100}% - 1.7%)` }}
          onMouseDown={() => note <= 95 && onNote(note)} />,
      );
    });
  }
  return <div className="piano" aria-label="Piano keyboard">{keys}{blacks}</div>;
}
