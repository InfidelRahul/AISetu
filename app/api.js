import { getBase } from "./platform.js";

function url(path) {
  const base = getBase();
  if (!base) return path;
  if (path.startsWith("http")) return path;
  return base + path;
}

export async function api(path, opts = {}) {
  const res = await fetch(url(path), {
    headers: { "Content-Type": "application/json", ...(opts.headers || {}) },
    ...opts,
    body: opts.body ? JSON.stringify(opts.body) : undefined,
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error?.message || res.statusText);
  return data;
}

export async function streamChat({ model, mode, messages, onDelta }) {
  const res = await fetch(url("/v1/chat/completions"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model, mode, stream: true, messages }),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error?.message || res.statusText);
  }
  if (res.body?.getReader) {
    const reader = res.body.getReader();
    const dec = new TextDecoder();
    let buf = "";
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      buf = emitSse(buf, onDelta);
    }
    return;
  }
  const text = await res.text();
  emitSse(text, onDelta);
}

function emitSse(buf, onDelta) {
  const parts = buf.split("\n\n");
  const rest = parts.pop() || "";
  for (const part of parts) {
    const line = part.split("\n").find((l) => l.startsWith("data: "));
    if (!line) continue;
    const payload = line.slice(6);
    if (payload === "[DONE]") continue;
    try {
      const json = JSON.parse(payload);
      const d = json.choices?.[0]?.delta?.content;
      if (d) onDelta(d);
    } catch {
      /* ignore */
    }
  }
  return rest;
}
