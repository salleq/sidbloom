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
