import type { Song } from "./model";

const inTauri = () => "__TAURI_INTERNALS__" in window;

/** Saves bytes with a native dialog in the desktop app, or as a browser download. */
export async function saveFile(name: string, bytes: Uint8Array | string, ext: string): Promise<boolean> {
  const data = typeof bytes === "string" ? new TextEncoder().encode(bytes) : bytes;
  if (inTauri()) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({ defaultPath: name, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
    if (!path) return false;
    await writeFile(path, data);
    return true;
  }
  const url = URL.createObjectURL(new Blob([data as BlobPart]));
  const a = Object.assign(document.createElement("a"), { href: url, download: name });
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}

/** Opens a .sidb project via the system file picker. */
export function openProject(): Promise<{ song: Song; name: string } | null> {
  return new Promise((resolve) => {
    const input = Object.assign(document.createElement("input"), { type: "file", accept: ".sidb,.json" });
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      try {
        const song = JSON.parse(await f.text()) as Song;
        if (!song.patterns || !song.orders || !song.instruments) throw new Error("not a SIDbloom project");
        resolve({ song, name: f.name });
      } catch (e) {
        alert(`Could not open ${f.name}: ${e}`);
        resolve(null);
      }
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}
