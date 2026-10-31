import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ChordPanel, FilterPanel, HelpPanel, InstrumentPanel, OrderPanel } from "./components/Panels";
import { Tracker } from "./components/Tracker";
import { Piano, Scope } from "./components/Widgets";
import { AudioEngine, type Positions } from "./audio";
import { createMidiBackend, trackerNote, type MidiNote } from "./midi";
import { confirmDiscard, openProject, pickMidi, pickSwm, saveFile, saveProject, type SaveTarget } from "./files";
import { MidiDialog } from "./components/MidiDialog";
import { NOTE_OFF, hex, noteFromKey, patternLoop, silenced, type Song } from "./model";
import { selRect, useStore } from "./store";
import { encodeMp3 } from "./mp3";
import { exportPrg, exportSid, renderSong, exportSwm, importMidi, importSwm, midiParts, swmExportNotes, newSong, projectFits, validate, type MidiPart } from "./wasm";

const isTyping = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA") && (t as HTMLInputElement).type !== "checkbox" && (t as HTMLInputElement).type !== "range";

const slug = (s: string) => s.trim().replace(/[^a-z0-9-_]+/gi, "_").replace(/^_+|_+$/g, "") || "song";

const TABS = [["song", "Song"], ["inst", "Instrument"], ["chords", "Chords"], ["filter", "Filter"], ["help", "Help"]] as const;
type TabId = (typeof TABS)[number][0];

export function App({ initial }: { initial: Song }) {
  const [st, dispatch] = useStore(initial);
  const audio = useMemo(() => new AudioEngine(), []);
  const [playPos, setPlayPos] = useState<Positions | null>(null);
  const [message, setMessage] = useState("");
  const [theme, setTheme] = useState<"auto" | "light" | "dark">("auto");
  const stateRef = useRef(st);
  stateRef.current = st;
  const playingRef = useRef(false);

  useEffect(() => {
    audio.onError = (m) => setMessage(`Audio error: ${m}`);
    (window as unknown as { __sidbloom: unknown }).__sidbloom = audio; // handy for end-to-end tests
  }, [audio]);

  // build the SID chip of the chosen type / video standard while idle, not when Play is pressed
  useEffect(() => { audio.prepare(st.sid8580, st.song.pal); }, [audio, st.sid8580, st.song.pal]);

  // open the audio output on the first click or key, so it has settled by the time Play is pressed
  useEffect(() => {
    const prime = () => audio.prime();
    window.addEventListener("pointerdown", prime, { once: true });
    window.addEventListener("keydown", prime, { once: true });
    return () => { window.removeEventListener("pointerdown", prime); window.removeEventListener("keydown", prime); };
  }, [audio]);

  useEffect(() => { document.documentElement.dataset.theme = theme === "auto" ? "" : theme; }, [theme]);
  useEffect(() => {
    if (!st.dirty) return;
    const warn = (e: BeforeUnloadEvent) => { e.preventDefault(); };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [st.dirty]);
  useEffect(() => { document.title = `${st.dirty ? "• " : ""}${st.song.title || "Untitled"} — SIDbloom`; }, [st.dirty, st.song.title]);

  const [tab, setTab] = useState<TabId>(() => {
    try { const t = localStorage.getItem("sidbloom.tab"); if (TABS.some(([id]) => id === t)) return t as TabId; } catch { /* private mode */ }
    return "song";
  });
  const chooseTab = (t: TabId) => {
    setTab(t);
    try { localStorage.setItem("sidbloom.tab", t); } catch { /* private mode */ }
  };

  const playing = playPos !== null;
  playingRef.current = playing;
  type PlayMode = "pattern" | "song" | "here" | "cursor";
  const modeRef = useRef<PlayMode>("song");
  const loopPos = useRef(0);
  // what the engine plays: the whole song, or just the order position that is looped
  // voices that are muted, or not soloed while another voice is, are silent (playback only; exports are not affected)
  const [mute, setMute] = useState([false, false, false]);
  const [solo, setSolo] = useState([false, false, false]);
  const audibleRef = useRef([true, true, true]);
  const audible = solo.some(Boolean) ? solo : mute.map((m) => !m);
  audibleRef.current = audible;
  const playable = useCallback((song: Song) => silenced(modeRef.current === "pattern" ? patternLoop(song, loopPos.current) : song, audibleRef.current), []);

  const sentSong = useRef<Song | null>(null); // the song the player already has
  const play = useCallback(async (mode: PlayMode, pos?: number) => {
    const s = stateRef.current;
    const p = pos ?? s.orderPos;
    modeRef.current = mode;
    loopPos.current = p;
    setMessage("");
    const [order, row] = mode === "cursor" ? [p, s.cursor.row] : mode === "here" ? [p, 0] : [0, 0];
    setPlayPos([[mode === "pattern" ? p : order, row], [0, 0], [0, 0]]);
    sentSong.current = s.song;
    await audio.play(playable(s.song), s.sid8580, mode === "pattern" ? 0 : order, row);
  }, [audio, playable]);
  const stop = useCallback(() => { audio.stop(); setPlayPos(null); }, [audio]);

  // a looped pattern reports order position 0; show it at the position that is looped
  useEffect(() => {
    audio.onPositions = (p) => setPlayPos(modeRef.current === "pattern" ? p.map(([, r]) => [loopPos.current, r] as [number, number]) : p);
  }, [audio]);

  // follow the playhead through the order list; a looped pattern follows the selected position instead
  useEffect(() => {
    if (!playPos) return;
    if (modeRef.current !== "pattern") {
      if (playPos[0][0] !== st.orderPos) dispatch({ t: "orderPos", pos: playPos[0][0] });
    } else if (st.orderPos !== loopPos.current) {
      void play("pattern", st.orderPos);
    }
  }, [playPos, st.orderPos, dispatch, play]);

  // ---- MIDI keyboard
  const midi = useMemo(() => createMidiBackend(), []);
  const [midiPorts, setMidiPorts] = useState<string[]>([]);
  const [midiPort, setMidiPort] = useState("");
  const [midiRec, setMidiRec] = useState(true);
  const midiRecRef = useRef(midiRec);
  midiRecRef.current = midiRec;

  const onMidiNote = useCallback((e: MidiNote) => {
    if (!e.on) return;
    const note = trackerNote(e.note);
    if (note === null) return;
    const s = stateRef.current;
    if (!playingRef.current) void audio.preview(s.song, s.curInst, note, s.sid8580);
    if (midiRecRef.current && !playingRef.current) dispatch({ t: "enterNote", note });
  }, [audio, dispatch]);

  // `silent`: called at start-up, where a refused permission must not produce a message
  const refreshMidi = useCallback(async (silent = false) => {
    if (!midi) return;
    try {
      const ports = await midi.ports();
      setMidiPorts(ports);
      setMidiPort((cur) => (cur && !ports.includes(cur) ? "" : cur));
    } catch (e) {
      if (!silent) setMessage(`MIDI unavailable: ${e instanceof Error ? e.message : e}`);
    }
  }, [midi]);

  const chooseMidi = useCallback(async (name: string) => {
    if (!midi) return;
    try {
      if (!name) { await midi.close(); setMidiPort(""); } else { await midi.open(name, onMidiNote); setMidiPort(name); setMessage(`MIDI input: ${name}`); }
      try { localStorage.setItem("sidbloom.midi", name); } catch { /* private mode */ }
    } catch (e) {
      setMidiPort("");
      setMessage(`Cannot open MIDI input: ${e instanceof Error ? e.message : e}`);
    }
  }, [midi, onMidiNote]);

  useEffect(() => {
    if (!midi) return;
    void (async () => {
      // A browser asks the user for permission the first time MIDI is used; do not ask at start-up.
      // The desktop app needs no permission; a browser that already granted it can list the devices.
      let allowed = midi.kind === "tauri";
      if (!allowed) {
        try { allowed = (await navigator.permissions.query({ name: "midi" as PermissionName })).state === "granted"; } catch { /* unsupported */ }
      }
      if (!allowed) return;
      await refreshMidi(true);
      let saved = "";
      try { saved = localStorage.getItem("sidbloom.midi") ?? ""; } catch { /* ignore */ }
      if (saved) {
        const ports = await midi.ports().catch(() => [] as string[]);
        if (ports.includes(saved)) await chooseMidi(saved);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [midi]);

  // muting or soloing while playing is heard at once
  const heardKey = audible.join();
  const sentKey = useRef(heardKey);
  useEffect(() => {
    if (heardKey === sentKey.current) return;
    sentKey.current = heardKey;
    if (playingRef.current) { sentSong.current = stateRef.current.song; audio.update(playable(stateRef.current.song)); }
  }, [heardKey, audio, playable]);

  // editing while playing: hand the changed song to the running player (debounced)
  useEffect(() => {
    if (!playing || st.song === sentSong.current) return;
    const t = window.setTimeout(() => { sentSong.current = st.song; audio.update(playable(st.song)); }, 120);
    return () => window.clearTimeout(t);
  }, [st.song, playing, audio, playable]);

  const hearNote = useCallback((note: number) => {
    const s = stateRef.current;
    if (!playing) void audio.preview(s.song, s.curInst, note, s.sid8580);
  }, [audio, playing]);

  const target = useRef<SaveTarget | null>(null);
  const save = useCallback(async (saveAs = false) => {
    const s = stateRef.current;
    const name = s.fileName.endsWith(".sidb") ? s.fileName : `${slug(s.song.title)}.sidb`;
    try {
      const r = await saveProject(name, JSON.stringify(s.song, null, 2), target.current, saveAs);
      if (!r) return;
      target.current = r.target;
      dispatch({ t: "saved", fileName: r.name });
      setMessage(`Saved ${r.name}`);
    } catch (e) {
      setMessage(`Cannot save: ${e instanceof Error ? e.message : e}`);
    }
  }, [dispatch]);
  const open = useCallback(async () => {
    const r = await openProject();
    if (r && (!stateRef.current.dirty || await confirmDiscard("Opening this file"))) { stop(); target.current = r.target; dispatch({ t: "load", song: r.song, fileName: r.name }); }
  }, [dispatch, stop]);
  const startNew = useCallback(async () => {
    if (stateRef.current.dirty && !(await confirmDiscard("Starting a new song"))) return;
    stop();
    target.current = null;
    dispatch({ t: "load", song: newSong() });
  }, [dispatch, stop]);

  const [notes, setNotes] = useState<string[]>([]);
  const [notesAbout, setNotesAbout] = useState("imported song");
  const importSidWizard = useCallback(async () => {
    const f = await pickSwm();
    if (!f) return;
    try {
      const r = importSwm(f.bytes);
      if (stateRef.current.dirty && !(await confirmDiscard("Importing this module"))) return;
      stop();
      target.current = null;
      dispatch({ t: "load", song: r.song, fileName: f.name.replace(/\.(swm|prg|p00)$/i, "") + ".sidb" });
      setNotes(r.warnings);
      setNotesAbout("imported song");
      setMessage(`Imported ${f.name}${r.warnings.length ? ` — ${r.warnings.length} things were approximated` : ""}`);
    } catch (e) {
      setMessage(`Cannot import ${f.name}: ${e instanceof Error ? e.message : e}`);
    }
  }, [dispatch, stop]);

  const [midiImport, setMidiImport] = useState<{ name: string; bytes: Uint8Array; parts: MidiPart[]; initial: (number | null)[] } | null>(null);
  const startMidiImport = useCallback(async () => {
    const f = await pickMidi();
    if (!f) return;
    try {
      const r = midiParts(f.bytes);
      setMidiImport({ name: f.name, bytes: f.bytes, parts: r.parts, initial: r.default });
    } catch (e) {
      setMessage(`Cannot import ${f.name}: ${e instanceof Error ? e.message : e}`);
    }
  }, []);
  const finishMidiImport = useCallback(async (voices: (number | null)[]) => {
    const m = midiImport;
    setMidiImport(null);
    if (!m) return;
    try {
      const r = importMidi(m.bytes, voices);
      if (stateRef.current.dirty && !(await confirmDiscard("Importing this MIDI file"))) return;
      stop();
      target.current = null;
      dispatch({ t: "load", song: r.song, fileName: m.name.replace(/\.(mid|midi|rmi)$/i, "") + ".sidb" });
      setNotes(r.warnings);
      setNotesAbout("imported song");
      setMessage(`Imported ${m.name}${r.warnings.length ? ` — ${r.warnings.length} notes about the conversion` : ""}`);
    } catch (e) {
      setMessage(`Cannot import ${m.name}: ${e instanceof Error ? e.message : e}`);
    }
  }, [midiImport, dispatch, stop]);

  const doExportSwm = useCallback(async () => {
    const s = stateRef.current;
    try {
      const bytes = exportSwm(s.song);
      const name = `${slug(s.song.title).slice(0, 12)}.swm.prg`; // SID-Wizard shows file names of up to 16 characters
      if (!(await saveFile(name, bytes, "prg"))) return;
      const left = swmExportNotes(s.song);
      setNotes(left);
      setNotesAbout("SID-Wizard export");
      setMessage(`Exported ${name} (${bytes.length} bytes)${left.length ? ` — ${left.length} things were left out or approximated` : ""}`);
    } catch (e) {
      setMessage(`Cannot export for SID-Wizard: ${e instanceof Error ? e.message : e}`);
    }
  }, []);

  const doExportMp3 = useCallback(async () => {
    const s = stateRef.current;
    const MAX = 600; // seconds
    try {
      setMessage("Rendering MP3…");
      await new Promise((r) => setTimeout(r, 30)); // let the message show before the work starts
      const pcm = renderSong(s.song, s.sid8580, MAX);
      const bytes = await encodeMp3(pcm);
      const name = `${slug(s.song.title)}.mp3`;
      const cut = pcm.length >= MAX * 44100;
      if (await saveFile(name, bytes, "mp3")) {
        setMessage(`Exported ${name} (${(pcm.length / 44100).toFixed(1)} s, ${(bytes.length / 1024).toFixed(0)} KiB)${cut ? " — cut off at 10 minutes" : ""}`);
      } else setMessage("");
    } catch (e) {
      setMessage(`Cannot export MP3: ${e instanceof Error ? e.message : e}`);
    }
  }, []);

  const doExport = useCallback(async (kind: "sid" | "prg") => {
    const s = stateRef.current;
    const err = validate(s.song);
    if (err) { setMessage(`Cannot export: ${err}`); return; }
    const bytes = kind === "sid" ? exportSid(s.song, s.sid8580) : exportPrg(s.song);
    const fits = projectFits(s.song);
    if (await saveFile(`${slug(s.song.title)}.${kind}`, bytes, kind)) {
      setMessage(`Exported ${slug(s.song.title)}.${kind} (${bytes.length} bytes)${fits ? "" : " — too large to embed the project, so it cannot be opened for editing again"}`);
    }
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTyping(e.target)) return;
      const s = stateRef.current;
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key;
      const lower = key.toLowerCase();
      const act = (a: Parameters<typeof dispatch>[0]) => { e.preventDefault(); dispatch(a); };
      if (mod) {
        if (lower === "z") return act({ t: e.shiftKey ? "redo" : "undo" });
        if (lower === "y") return act({ t: "redo" });
        if (lower === "s") { e.preventDefault(); void save(e.shiftKey); return; }
        if (lower === "o") { e.preventDefault(); void open(); return; }
        if (lower === "c" || lower === "x") return act({ t: "copy", cut: lower === "x" });
        if (lower === "v") return act({ t: "paste" });
        if (lower === "a") return act({ t: "selectAll" });
        if (key === "[" || key === "]" || key === "{" || key === "}") return act({ t: "transpose", d: (key === "[" || key === "{" ? -1 : 1) * (key === "{" || key === "}" ? 12 : 1) });
        if (key === "ArrowLeft") return act({ t: "orderPos", pos: s.orderPos - 1 });
        if (key === "ArrowRight") return act({ t: "orderPos", pos: s.orderPos + 1 });
        return;
      }
      if (e.altKey) return;
      switch (key) {
        case " ": e.preventDefault(); return void (playing ? stop() : play(e.shiftKey ? "song" : "pattern"));
        case "Enter": e.preventDefault(); return void play(e.shiftKey ? "here" : "cursor");
        case "Escape": dispatch({ t: "deselect" }); return stop();
        case "ArrowUp": return act({ t: "move", drow: -1, extend: e.shiftKey });
        case "ArrowDown": return act({ t: "move", drow: 1, extend: e.shiftKey });
        case "ArrowLeft": return act({ t: "move", dfield: -1, extend: e.shiftKey });
        case "ArrowRight": return act({ t: "move", dfield: 1, extend: e.shiftKey });
        case "Tab": return act({ t: "move", dch: e.shiftKey ? -1 : 1 });
        case "PageUp": return act({ t: "move", drow: -16 });
        case "PageDown": return act({ t: "move", drow: 16 });
        case "Home": return act({ t: "cursor", c: { row: 0 } });
        case "End": return act({ t: "cursor", c: { row: 999 } });
        case "Delete": return act({ t: "clearCell" });
        case "Backspace": return act({ t: e.shiftKey ? "insertRow" : "clearCell", ...(e.shiftKey ? { remove: true } : { backspace: true }) } as never);
        case "Insert": return act({ t: "insertRow" });
        case "[": case "<": case "F1": return act({ t: "octave", d: -1 });
        case "]": case ">": case "F2": return act({ t: "octave", d: 1 });
        case "F3": return act({ t: "curInst", i: s.curInst - 1 });
        case "F4": return act({ t: "curInst", i: s.curInst + 1 });
        case "-": return act({ t: "curInst", i: s.curInst - 1 });
        case "=": if (s.cursor.field !== 0) return act({ t: "curInst", i: s.curInst + 1 }); break;
      }
      if (s.cursor.field === 0) {
        if (key === "1" || key === "`") {
          e.preventDefault();
          return dispatch({ t: "enterNote", note: NOTE_OFF });
        }
        const note = noteFromKey(key, s.octave);
        if (note !== null && !e.shiftKey) {
          e.preventDefault();
          dispatch({ t: "enterNote", note });
          hearNote(note);
        }
      } else if (/^[0-9a-f]$/i.test(key)) {
        e.preventDefault();
        dispatch({ t: "enterHex", digit: parseInt(key, 16) });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [dispatch, hearNote, open, play, playing, save, stop]);

  const playRows = useMemo(
    () => [0, 1, 2].map((ch) => (playPos && playPos[ch][0] === st.orderPos ? playPos[ch][1] : null)),
    [playPos, st.orderPos],
  );
  const song = st.song;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand"><img className="logo" src="/favicon.svg" alt="" width={26} height={26} /> SIDbloom</div>
        <div className="group">
          <button onClick={() => void startNew()} title="Start a new song (asks first if there are unsaved changes)">New</button>
          <button onClick={() => void open()} title="Open a .sidb project, or a .sid / .prg exported by SIDbloom (Ctrl+O)">Open</button>
          <button onClick={() => void save()} title="Save the project (Ctrl+S); overwrites the file it was opened from or last saved to">Save</button>
          <button onClick={() => void save(true)} title="Save under a new name (Ctrl+Shift+S)" data-testid="save-as">Save As…</button>
          <button onClick={() => void importSidWizard()} title="Import a SID-Wizard 1.x module (.swm)" data-testid="import-swm">Import SW…</button>
          <button onClick={() => void startMidiImport()} title="Import a MIDI file (.mid)" data-testid="import-mid">Import MIDI…</button>
        </div>
        <div className="group">
          <button onClick={() => dispatch({ t: "undo" })} disabled={!st.past.length} title="Undo (Ctrl+Z)" aria-label="Undo">↶</button>
          <button onClick={() => dispatch({ t: "redo" })} disabled={!st.future.length} title="Redo (Ctrl+Y)" aria-label="Redo">↷</button>
        </div>
        <div className="group transport">
          <button className="primary" onClick={() => (playing ? stop() : void play("pattern"))} data-testid="play" aria-label={playing ? "Stop" : "Play"} title="Loop the current pattern / stop (Space)">
            {playing ? "■ Stop" : "▶ Pattern"}
          </button>
          <button onClick={() => void play("song")} title="Play the whole song from the start (Shift+Space)" data-testid="play-song">▶ Song</button>
          <button onClick={() => void play("here")} title="Play the song from the start of the current pattern (Shift+Enter)" data-testid="play-from-pattern">▶ From pattern</button>
          <button onClick={() => void play("cursor")} title="Play the song from the cursor (Enter)">▶ From cursor</button>
        </div>
        <div className="group">
          <label className="inline">Speed
            <input type="number" min={1} max={31} value={song.speed} onChange={(e) => dispatch({ t: "edit", fn: (s) => { s.speed = Math.max(1, Math.min(31, Number(e.target.value) || 6)); } })} />
          </label>
          <select aria-label="Video standard" value={song.pal ? "pal" : "ntsc"} onChange={(e) => dispatch({ t: "edit", fn: (s) => { s.pal = e.target.value === "pal"; } })}>
            <option value="pal">PAL 50 Hz</option><option value="ntsc">NTSC 60 Hz</option>
          </select>
          <select aria-label="SID chip" value={st.sid8580 ? "8580" : "6581"} onChange={(e) => dispatch({ t: "sid8580", v: e.target.value === "8580" })}>
            <option value="8580">SID 8580</option><option value="6581">SID 6581</option>
          </select>
        </div>
        {midi && (
          <div className="group" title="Play notes from a MIDI keyboard">
            <select aria-label="MIDI input" value={midiPort} onFocus={() => void refreshMidi()} onMouseDown={() => void refreshMidi()} onChange={(e) => void chooseMidi(e.target.value)}>
              <option value="">{midiPorts.length || midi.kind === "tauri" ? "MIDI: off" : "MIDI: choose…"}</option>
              {midiPorts.map((p) => <option key={p} value={p}>{p}</option>)}
            </select>
            <label className="chip"><input type="checkbox" checked={midiRec} onChange={() => setMidiRec(!midiRec)} aria-label="Record MIDI notes into the pattern" /><span title="Write played notes into the pattern at the cursor">Rec</span></label>
          </div>
        )}
        <div className="group right">
          <button onClick={() => void doExport("sid")} className="accent" data-testid="export-sid">Export .sid</button>
          <button onClick={() => void doExport("prg")}>.prg</button>
          <button onClick={() => void doExportMp3()} title="Render the song once through to an MP3 audio file (with the chosen SID chip)" data-testid="export-mp3">.mp3</button>
          <button onClick={() => void doExportSwm()} title="Export a SID-Wizard 1.x module (.swm.prg) to continue the tune in SID-Wizard" data-testid="export-swm">SW .swm</button>
          <button onClick={() => setTheme(theme === "dark" ? "light" : theme === "light" ? "auto" : "dark")} title="Theme" aria-label="Toggle theme">◑ {theme}</button>
        </div>
      </header>

      <div className="meta">
        <input aria-label="Title" placeholder="Title" value={song.title} maxLength={31} onChange={(e) => dispatch({ t: "edit", fn: (s) => { s.title = e.target.value; } })} />
        <input aria-label="Author" placeholder="Author" value={song.author} maxLength={31} onChange={(e) => dispatch({ t: "edit", fn: (s) => { s.author = e.target.value; } })} />
        <input aria-label="Released" placeholder="Released (e.g. 2026 SIDbloom)" value={song.released} maxLength={31} onChange={(e) => dispatch({ t: "edit", fn: (s) => { s.released = e.target.value; } })} />
        <span className="status" role="status" data-testid="status">{message}</span>
      </div>

      {notes.length > 0 && (
        <details className="import-notes" data-testid="import-notes">
          <summary>{notes.length} notes about the {notesAbout} (things approximated or left out)</summary>
          <ul>{notes.map((n, i) => <li key={i}>{n}</li>)}</ul>
          <button onClick={() => setNotes([])}>Dismiss</button>
        </details>
      )}

      {midiImport && <MidiDialog fileName={midiImport.name} parts={midiImport.parts} initial={midiImport.initial} onImport={finishMidiImport} onCancel={() => setMidiImport(null)} />}

      <main className="main">
        <div className="left">
          <Tracker song={song} orderPos={st.orderPos} cursor={st.cursor} sel={st.sel ? selRect(st) : null} playRows={playRows} onCursor={(c, extend) => dispatch({ t: "cursor", c, extend })}
            audible={audible} mute={mute} solo={solo}
            onMute={(ch) => setMute(mute.map((m, c) => (c === ch ? !m : m)))} onSolo={(ch) => setSolo(solo.map((m, c) => (c === ch ? !m : m)))}
            canPaste={!!st.clip} onCopyTrack={(ch) => dispatch({ t: "copyTrack", ch })} onPasteTrack={(ch) => dispatch({ t: "pasteTrack", ch })} />
          <div className="statusbar">
            <span>Pos <b>{hex(st.orderPos)}</b></span>
            <span>Row <b>{hex(st.cursor.row)}</b></span>
            <span className="btn-row">Octave
              <button className="mini" onClick={() => dispatch({ t: "octave", d: -1 })} aria-label="Octave down" title="Octave down (&lt; or F1)">−</button>
              <b data-testid="octave">{st.octave}</b>
              <button className="mini" onClick={() => dispatch({ t: "octave", d: 1 })} aria-label="Octave up" title="Octave up (&gt; or F2)">+</button>
            </span>
            <span>Instrument <b>{hex(st.curInst)}</b></span>
            <span className="btn-row">
              <button onClick={() => dispatch({ t: "copy" })} title="Copy the selected cells (Ctrl+C). Select with the mouse or Shift+arrows.">Copy</button>
              <button onClick={() => dispatch({ t: "copy", cut: true })} title="Cut (Ctrl+X)">Cut</button>
              <button onClick={() => dispatch({ t: "paste" })} disabled={!st.clip} title="Paste at the cursor (Ctrl+V)">Paste</button>
            </span>
            <label className="inline">Step <input type="number" min={0} max={16} value={st.step} onChange={(e) => dispatch({ t: "step", n: Number(e.target.value) })} /></label>
            <Scope analyser={audio.analyser} />
          </div>
          <Piano octave={st.octave} onNote={(note) => { hearNote(note); if (st.cursor.field === 0) dispatch({ t: "enterNote", note }); }} />
        </div>
        <aside className="right-col">
          <nav className="tabs" role="tablist" aria-label="Panels">
            {TABS.map(([id, label]) => (
              <button key={id} role="tab" aria-selected={tab === id} className={`tab${tab === id ? " on" : ""}`} onClick={() => chooseTab(id)} data-testid={`tab-${id}`}>{label}</button>
            ))}
          </nav>
          <div className="tab-body">
            {tab === "song" && <OrderPanel song={song} orderPos={st.orderPos} ch={st.cursor.ch} dispatch={dispatch} playOrder={playPos ? playPos[0][0] : null} onPlay={(p) => { dispatch({ t: "orderPos", pos: p }); void play("pattern", p); }} />}
            {tab === "inst" && <InstrumentPanel song={song} inst={st.curInst} clip={st.instClip} dispatch={dispatch} onPreview={(note) => hearNote(note ?? 58)} />}
            {tab === "chords" && <ChordPanel song={song} dispatch={dispatch} />}
            {tab === "filter" && <FilterPanel song={song} dispatch={dispatch} />}
            {tab === "help" && <HelpPanel />}
          </div>
        </aside>
      </main>
    </div>
  );
}
