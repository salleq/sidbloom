// Regenerates the README screenshots in docs/screenshots (run after `npm run build`).
import { chromium } from "playwright";
import { preview } from "vite";
import { mkdirSync } from "node:fs";

const out = process.env.SHOTS_DIR ?? "../docs/screenshots";
mkdirSync(out, { recursive: true });

const server = await preview({ preview: { port: 4174, strictPort: true } });
const url = "http://localhost:4174/";
const args = ["--autoplay-policy=no-user-gesture-required"];
let browser;
try {
  try { browser = await chromium.launch({ args }); }
  catch { browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: [...args, "--no-sandbox"] }); }

  const open = async (colorScheme, height = 900) => {
    const ctx = await browser.newContext({ viewport: { width: 1440, height }, colorScheme });
    const page = await ctx.newPage();
    await page.goto(url);
    await page.waitForSelector("[data-testid=tracker]");
    return page;
  };
  // play the demo song for a moment so the playhead and oscilloscope show activity
  const playDemo = async (page) => {
    await page.getByTestId("play").click();
    await page.waitForFunction(() => window.__sidbloom.messages > 3, null, { timeout: 15000 });
    await page.waitForTimeout(2500);
  };

  for (const scheme of ["light", "dark"]) {
    const page = await open(scheme);
    await playDemo(page);
    await page.screenshot({ path: `${out}/editor-${scheme}.png` });
    await page.context().close();
  }

  // instrument, chord and filter panels: tall window, clip to the right column
  {
    const page = await open("light", 1650);
    await page.locator(".right-col").screenshot({ path: `${out}/instrument-filter.png` });
    await page.context().close();
  }

  // MIDI import dialog with a tiny two-part file
  {
    const page = await open("light");
    const vlq = (n) => { const o = [n & 0x7f]; n >>= 7; while (n) { o.unshift((n & 0x7f) | 0x80); n >>= 7; } return o; };
    const trk = (ev) => { const b = ev.flatMap(([d, ...e]) => [...vlq(d), ...e]).concat([0, 0xff, 0x2f, 0]); return Buffer.concat([Buffer.from("MTrk"), Buffer.from([0, 0, b.length >> 8, b.length & 255]), Buffer.from(b)]); };
    const midi = Buffer.concat([
      Buffer.from([0x4d, 0x54, 0x68, 0x64, 0, 0, 0, 6, 0, 1, 0, 2, 1, 0xe0]),
      trk([[0, 0xff, 0x03, 4, 0x4c, 0x65, 0x61, 0x64], [0, 0xff, 0x51, 3, 7, 0xa1, 0x20], [0, 0x90, 69, 100], [480, 0x80, 69, 0], [0, 0x90, 71, 100], [480, 0x80, 71, 0]]),
      trk([[0, 0xff, 0x03, 4, 0x42, 0x61, 0x73, 0x73], [0, 0x91, 45, 100], [960, 0x81, 45, 0]]),
    ]);
    const [fc] = await Promise.all([page.waitForEvent("filechooser"), page.getByTestId("import-mid").click()]);
    await fc.setFiles({ name: "tune.mid", mimeType: "audio/midi", buffer: midi });
    await page.getByTestId("midi-dialog").waitFor();
    await page.screenshot({ path: `${out}/midi-import.png` });
    await page.context().close();
  }
  console.log("screenshots written to", out);
} finally {
  await browser?.close();
  server.httpServer.close();
}
