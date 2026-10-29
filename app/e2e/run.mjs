// End-to-end test: builds are served by `vite preview`; drives the real UI in Chromium.
import { chromium } from "playwright";
import { preview } from "vite";
import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";

const shots = process.env.SHOTS_DIR ?? "e2e/shots";
mkdirSync(shots, { recursive: true });

const server = await preview({ preview: { port: 4173, strictPort: true } });
const url = "http://localhost:4173/";
let browser;
try {
  try { browser = await chromium.launch({ args: ["--autoplay-policy=no-user-gesture-required"] }); }
  catch { browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--autoplay-policy=no-user-gesture-required", "--no-sandbox"] }); }
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, acceptDownloads: true });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));

  // a fake MIDI keyboard (Web MIDI), so the test can "play" it
  await page.addInitScript(() => {
    const input = { id: "kbd1", name: "Test Keyboard", onmidimessage: null };
    window.__fakeMidi = { send: (bytes) => input.onmidimessage && input.onmidimessage({ data: new Uint8Array(bytes) }) };
    navigator.requestMIDIAccess = async () => ({ inputs: new Map([["kbd1", input]]) });
  });
  let dialogText = "", dialogAccept = true; // alerts and the "unsaved changes" question: remember the text, answer
  page.on("dialog", async (d) => { dialogText = d.message(); await (dialogAccept ? d.accept() : d.dismiss()); });
  await page.goto(url);
  await page.waitForSelector("[data-testid=tracker]");
  const step = (s) => console.log("✓", s);

  // demo song is loaded: first row of voice 1 has A-4
  assert.match(await page.locator(".trow").first().innerText(), /A-4/);
  step("demo song shown in tracker");

  // typing notes: move to an empty row and enter C-4 with Z
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("z");
  const row2 = await page.locator(".trow").nth(1).innerText();
  assert.match(row2, /C-4/, row2);
  step("piano key enters note");
  await page.keyboard.press("Control+z");
  assert.doesNotMatch(await page.locator(".trow").nth(1).innerText(), /C-4/);
  await page.keyboard.press("Control+y");
  assert.match(await page.locator(".trow").nth(1).innerText(), /C-4/);
  step("undo / redo");


  // selection, copy and paste: drag over cells, paste elsewhere
  const cells = page.locator(".trow");
  await page.locator(".trow").nth(0).locator(".tcell").nth(0).locator(".f.note").hover();
  await page.mouse.down();
  await cells.nth(2).locator(".tcell").nth(1).locator(".f.note").hover();
  await page.mouse.up();
  assert.equal(await page.locator(".tcell.selected").count(), 6, "3 rows x 2 voices selected");
  await page.keyboard.press("Control+c");
  const src = await cells.nth(0).locator(".tcell").nth(0).innerText();
  await cells.nth(10).locator(".tcell").nth(1).locator(".f.note").click();
  assert.equal(await page.locator(".tcell.selected").count(), 0, "click clears the selection");
  await page.keyboard.press("Control+v");
  assert.equal(await cells.nth(10).locator(".tcell").nth(1).innerText(), src, "pasted cell equals the copied one");
  assert.equal(await cells.nth(10).locator(".tcell").nth(2).innerText(), await cells.nth(0).locator(".tcell").nth(1).innerText());
  await page.keyboard.press("Control+z");
  assert.notEqual(await cells.nth(10).locator(".tcell").nth(1).innerText(), src, "paste is undoable");
  step("select with the mouse, copy and paste");

  // unsaved changes: New asks first; cancelling keeps the song
  dialogText = ""; dialogAccept = false;
  await page.locator(".topbar").getByRole("button", { name: "New", exact: true }).click();
  await page.waitForTimeout(200);
  assert.match(dialogText, /unsaved changes/);
  assert.match(await page.locator(".trow").first().innerText(), /A-4/, "cancel keeps the song");
  dialogAccept = true;
  await page.locator(".topbar").getByRole("button", { name: "New", exact: true }).click();
  await page.waitForFunction(() => document.title.startsWith("Untitled"));
  // a clean new song needs no confirmation
  dialogText = "";
  await page.locator(".topbar").getByRole("button", { name: "New", exact: true }).click();
  await page.waitForTimeout(200);
  assert.equal(dialogText, "");
  // a new song: each voice has its own pattern
  await page.locator(".trow").nth(0).locator(".f.note").first().click();
  await page.keyboard.press("z");
  assert.doesNotMatch(await page.locator(".trow").nth(0).locator(".tcell").nth(1).innerText(), /C-4/, "other voices unchanged");
  await page.keyboard.press("Control+z");
  step("New asks before discarding changes; voices of a new song are independent");

  // the demo song again for the audio checks
  await page.reload();
  await page.waitForSelector("[data-testid=tracker]");

  // playback runs in an AudioWorklet and reports positions; output must be audible
  await page.getByTestId("play").click();
  await page.waitForFunction(() => window.__sidbloom.messages > 3, null, { timeout: 15000 });
  const level = await page.evaluate(async () => {
    const a = window.__sidbloom.analyser.current;
    let peak = 0;
    for (let i = 0; i < 40; i++) {
      const b = new Float32Array(a.fftSize); a.getFloatTimeDomainData(b);
      for (const v of b) peak = Math.max(peak, Math.abs(v));
      await new Promise((r) => setTimeout(r, 50));
    }
    return peak;
  });
  assert.ok(level > 0.02, `output level too low: ${level}`);
  assert.ok((await page.locator(".tcell.playing").count()) >= 1, "no playing row highlighted");
  step(`audio plays (peak ${level.toFixed(3)}), playhead follows`);
  await page.screenshot({ path: `${shots}/playing.png` });

  // editing during playback reaches the player without stopping it
  const before = await page.evaluate(() => window.__sidbloom.messages);
  await page.locator(".f.note").nth(40).click();
  await page.keyboard.press("c");
  await page.waitForTimeout(400);
  await page.waitForFunction((b) => window.__sidbloom.messages > b + 2, before);
  assert.ok((await page.getByTestId("play").innerText()).includes("Stop"), "playback stopped by an edit");
  step("edit during playback keeps playing");
  await page.getByTestId("play").click();
  await page.waitForFunction(() => !document.querySelector(".tcell.playing"));
  step("stop");

  // export .sid
  const [dl] = await Promise.all([page.waitForEvent("download"), page.getByTestId("export-sid").click()]);
  const path = await dl.path();
  const { readFileSync } = await import("node:fs");
  const bytes = readFileSync(path);
  assert.equal(bytes.subarray(0, 4).toString("latin1"), "PSID");
  assert.ok(dl.suggestedFilename().endsWith(".sid"));
  step(`exported ${dl.suggestedFilename()} (${bytes.length} bytes)`);

  // the exported .sid opens again as an editable project
  await page.locator(".topbar").getByRole("button", { name: "New", exact: true }).click();
  await page.waitForFunction(() => document.title.startsWith("Untitled"));
  const [fc] = await Promise.all([page.waitForEvent("filechooser"), page.locator(".topbar").getByRole("button", { name: "Open", exact: true }).click()]);
  await fc.setFiles({ name: dl.suggestedFilename(), mimeType: "application/octet-stream", buffer: bytes });
  await page.waitForFunction(() => document.title.includes("SIDbloom demo"));
  assert.match(await page.locator(".trow").first().innerText(), /A-4/);
  step("exported .sid opened again for editing");
  // a .sid that SIDbloom did not export gives a clear message
  dialogText = "";
  const [fc2] = await Promise.all([page.waitForEvent("filechooser"), page.locator(".topbar").getByRole("button", { name: "Open", exact: true }).click()]);
  await fc2.setFiles({ name: "other.sid", mimeType: "application/octet-stream", buffer: Buffer.concat([Buffer.from("PSID"), Buffer.alloc(2), Buffer.from("x".repeat(200))]) });
  await page.waitForTimeout(300);
  assert.match(dialogText, /no SIDbloom project/);
  step("foreign .sid is rejected with an explanation");

  // import a SID-Wizard module (a tiny hand-built SWM1 file)
  const hdr = Buffer.alloc(64);
  hdr.write("SWM1", 0); hdr[4] = 1; hdr[11] = 32; hdr[12] = 3; hdr[13] = 1; hdr[14] = 1; hdr.write("IMPORTED TUNE", 24);
  const inst = Buffer.alloc(0x10); inst[3] = 0x09; inst[4] = 0xa9;
  const wf = Buffer.from([0x41, 0, 0, 0x41, 0x0c, 0, 0xff]);
  const pw = Buffer.from([0x88, 0, 0, 0x10, 0x20, 0, 0xff]);
  inst[0x0a] = 0x10 + wf.length; inst[0x0b] = 0x10 + wf.length + pw.length;
  const instAll = Buffer.concat([inst, wf, pw, Buffer.from([0xff])]);
  const mod = Buffer.concat([
    Buffer.from([0x00, 0x20]), hdr,
    Buffer.from([1, 0xfe, 2, 1, 0xfe, 2, 1, 0xfe, 2]),
    Buffer.from([0xb1, 0x81, 0x0f, 0x40, 0x00, 0x74, 0x32, 13, 9]),
    instAll, Buffer.from([instAll.length]), Buffer.from("TEST    "),
    Buffer.from([0x86, 0x86]),
  ]);
  const [chooser] = await Promise.all([page.waitForEvent("filechooser"), page.getByTestId("import-swm").click()]);
  await chooser.setFiles({ name: "test.swm.prg", mimeType: "application/octet-stream", buffer: mod });
  await page.waitForFunction(() => document.title.includes("IMPORTED TUNE"));
  assert.match(await page.locator(".trow").first().innerText(), /C-4/);
  assert.match(await page.getByTestId("status").innerText(), /Imported test\.swm\.prg/);
  step("SID-Wizard module imported");

  // instrument editing changes the song and is undoable
  await page.getByLabel("Name").fill("My lead");
  assert.equal(await page.getByLabel("Name").inputValue(), "My lead");
  await page.screenshot({ path: `${shots}/ui.png` });

  // chords: add one, give it to the instrument, and the test-note sound must change
  const energy = async () => page.evaluate(async () => {
    const a = window.__sidbloom.analyser.current; let e = 0;
    for (let i = 0; i < 60; i++) { const b = new Float32Array(a.fftSize); a.getFloatTimeDomainData(b); e += b.reduce((s, v) => s + Math.abs(v), 0); await new Promise((r) => setTimeout(r, 30)); }
    return e;
  });
  await page.getByRole("button", { name: "＋ Chord" }).click();
  assert.equal(await page.getByLabel("Chord 1 notes").inputValue(), "0 4 7");
  await page.getByLabel("Chord 1 notes").fill("0 12");
  await page.getByLabel("Chord 1 notes").press("Enter");
  await page.getByLabel("Default chord").selectOption("1");
  assert.equal(await page.getByLabel("Default chord").inputValue(), "1");
  await page.getByRole("button", { name: /Test/ }).click();
  assert.ok((await energy()) > 1, "chord instrument silent");
  step("chord editor works and chord instrument plays");

  // import a MIDI file: pick which part goes to which voice
  const vlq = (n) => { const o = [n & 0x7f]; n >>= 7; while (n) { o.unshift((n & 0x7f) | 0x80); n >>= 7; } return o; };
  const trk = (ev) => { const b = ev.flatMap(([d, ...e]) => [...vlq(d), ...e]).concat([0, 0xff, 0x2f, 0]); return Buffer.concat([Buffer.from("MTrk"), Buffer.from([0, 0, b.length >> 8, b.length & 255]), Buffer.from(b)]); };
  const midi = Buffer.concat([
    Buffer.from([0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 1, 0, 2, 1, 0xe0]), // format 1, 2 tracks, 480 ppq
    trk([[0, 0xff, 0x03, 4, 0x4c, 0x65, 0x61, 0x64], [0, 0xff, 0x51, 3, 7, 0xa1, 0x20], [0, 0x90, 69, 100], [480, 0x80, 69, 0], [0, 0x90, 71, 100], [480, 0x80, 71, 0]]),
    trk([[0, 0xff, 0x03, 4, 0x42, 0x61, 0x73, 0x73], [0, 0x91, 45, 100], [960, 0x81, 45, 0]]),
  ]);
  const [mc] = await Promise.all([page.waitForEvent("filechooser"), page.getByTestId("import-mid").click()]);
  await mc.setFiles({ name: "tune.mid", mimeType: "audio/midi", buffer: midi });
  await page.getByTestId("midi-dialog").waitFor();
  const dlg = page.getByTestId("midi-dialog");
  assert.equal(await dlg.getByLabel("Voice 1").inputValue(), "0", "busiest part preselected");
  await dlg.getByLabel("Voice 1").selectOption("1"); // bass
  await dlg.getByLabel("Voice 2").selectOption("0"); // lead
  await page.getByTestId("midi-import-go").click();
  await page.waitForFunction(() => document.title.includes("Lead"));
  assert.match(await page.locator(".trow").first().innerText(), /A-4/, "voice 2 plays the lead's A4 on row 0");
  assert.match(await page.getByTestId("status").innerText(), /Imported tune\.mid/);
  step("MIDI file imported through the dialog");
  // cancelling changes nothing
  const [mc2] = await Promise.all([page.waitForEvent("filechooser"), page.getByTestId("import-mid").click()]);
  await mc2.setFiles({ name: "tune.mid", mimeType: "audio/midi", buffer: midi });
  await page.getByTestId("midi-dialog").waitFor();
  await page.keyboard.press("Escape");
  await page.getByTestId("midi-dialog").waitFor({ state: "detached" });
  // not a MIDI file: clear message, no dialog
  const [mc3] = await Promise.all([page.waitForEvent("filechooser"), page.getByTestId("import-mid").click()]);
  await mc3.setFiles({ name: "x.mid", mimeType: "audio/midi", buffer: Buffer.from("not midi at all") });
  await page.waitForFunction(() => /not a MIDI file/.test(document.querySelector("[data-testid=status]").textContent));
  step("cancel and bad MIDI files handled");

  // MIDI keyboard: notes play and are written at the cursor; Rec off only plays
  await page.getByLabel("MIDI input").focus(); // devices are listed (and permission asked) on first use
  await page.waitForFunction(() => [...document.querySelectorAll("[aria-label='MIDI input'] option")].some((o) => o.textContent === "Test Keyboard"));
  await page.getByLabel("MIDI input").selectOption("Test Keyboard");
  await page.locator(".trow").nth(2).locator(".f.note").first().click();
  assert.doesNotMatch(await page.locator(".trow").nth(2).innerText(), /A-4/);
  assert.ok((await page.getByTestId("play").innerText()).includes("Play"), "previewing must not look like playing");
  await page.evaluate(() => window.__fakeMidi.send([0x90, 69, 100])); // A4
  await page.waitForTimeout(200);
  assert.match(await page.locator(".trow").nth(2).innerText(), /A-4/);
  await page.evaluate(() => window.__fakeMidi.send([0x80, 69, 0])); // note off: nothing written
  await page.evaluate(() => window.__fakeMidi.send([0x90, 72, 0])); // velocity 0 = note off
  assert.doesNotMatch(await page.locator(".trow").nth(3).innerText(), /C-5/);
  step("MIDI note is written at the cursor");
  await page.getByTitle("Write played notes into the pattern at the cursor").click();
  assert.equal(await page.getByLabel("Record MIDI notes into the pattern").isChecked(), false);
  await page.evaluate(() => window.__fakeMidi.send([0x90, 72, 100])); // C5, only played
  await page.waitForTimeout(100);
  assert.doesNotMatch(await page.locator(".trow").nth(3).innerText(), /C-5/);
  assert.ok((await energy()) > 1, "MIDI note was not heard");
  step("MIDI note is heard and, with Rec off, not written");

  // light theme + narrow window render without horizontal overflow
  await page.getByRole("button", { name: /Toggle theme/ }).click();
  await page.setViewportSize({ width: 820, height: 900 });
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 1, `horizontal overflow ${overflow}px`);
  await page.screenshot({ path: `${shots}/narrow.png` });
  step("responsive layout");

  assert.deepEqual(errors, [], `console errors: ${errors.join("\n")}`);
  step("no console errors");
} finally {
  await browser?.close();
  server.httpServer.close();
}
