/**
 * Context Builder — selects textual context compatible with the active mode.
 * Does not blindly send all available context.
 */

const MODE_KEYS = {
  ask: ["userRequest", "currentFile", "selection"],
  code: ["userRequest", "currentFile", "selection", "relatedCode", "project"],
  debug: ["userRequest", "errors", "terminal", "currentFile", "selection"],
  plan: ["userRequest", "project", "gitDiff"],
  review: ["userRequest", "gitDiff", "currentFile", "selection", "relatedCode"],
};

export function buildContext(mode, sources = {}) {
  const keys = MODE_KEYS[mode.id] || MODE_KEYS.ask;
  const parts = [];
  for (const key of keys) {
    const value = sources[key];
    if (value && String(value).trim()) {
      parts.push({ kind: key, text: String(value).trim() });
    }
  }
  return parts;
}

export function formatContext(parts) {
  if (!parts.length) return "";
  return parts.map((p) => `[${p.kind}]\n${p.text}`).join("\n\n");
}
