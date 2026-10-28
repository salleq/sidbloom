// AudioWorkletGlobalScope has no TextEncoder/TextDecoder; wasm-bindgen glue needs them at load time.
const g = globalThis as unknown as Record<string, unknown>;
if (typeof g.TextDecoder === "undefined") {
  g.TextDecoder = class {
    decode(buf?: Uint8Array): string {
      let s = "";
      if (buf) for (const b of buf) s += String.fromCharCode(b);
      return decodeURIComponent(escape(s));
    }
  };
}
if (typeof g.TextEncoder === "undefined") {
  g.TextEncoder = class {
    encode(s: string): Uint8Array {
      const bin = unescape(encodeURIComponent(s));
      return Uint8Array.from(bin, (c) => c.charCodeAt(0));
    }
    encodeInto(s: string, dst: Uint8Array) {
      const b = this.encode(s);
      dst.set(b.subarray(0, dst.length));
      return { read: s.length, written: Math.min(b.length, dst.length) };
    }
  };
}
export {};
