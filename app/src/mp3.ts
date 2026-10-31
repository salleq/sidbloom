/** MP3 encoding (LAME via lamejs, loaded only when an MP3 is exported). */
export const MP3_RATE = 44100;

export async function encodeMp3(pcm: Int16Array, kbps = 128): Promise<Uint8Array> {
  const { Mp3Encoder } = await import("@breezystack/lamejs");
  const enc = new Mp3Encoder(1, MP3_RATE, kbps);
  const parts: Uint8Array[] = [];
  const block = 1152 * 16;
  for (let i = 0; i < pcm.length; i += block) {
    const out = enc.encodeBuffer(pcm.subarray(i, i + block));
    if (out.length) parts.push(out);
    if ((i / block) % 8 === 7) await new Promise((r) => setTimeout(r)); // keep the window responsive
  }
  const end = enc.flush();
  if (end.length) parts.push(end);
  const bytes = new Uint8Array(parts.reduce((n, x) => n + x.length, 0));
  let at = 0;
  for (const x of parts) { bytes.set(x, at); at += x.length; }
  return bytes;
}
