import fs from "node:fs";
import path from "node:path";
import { EventEmitter } from "node:events";

const SECRET_KEYS = /password|secret|token|authorization|cookie|api[_-]?key/i;

export const logBus = new EventEmitter();
logBus.setMaxListeners(50);

const buffer = [];
const MAX_BUFFER = 2000;
let logFile = null;

export function initLogging(logDir) {
  fs.mkdirSync(logDir, { recursive: true });
  logFile = path.join(logDir, "aisetu.log");
}

function redact(obj) {
  if (obj == null || typeof obj !== "object") return obj;
  const out = Array.isArray(obj) ? [] : {};
  for (const [k, v] of Object.entries(obj)) {
    out[k] = SECRET_KEYS.test(k) ? "[redacted]" : typeof v === "object" ? redact(v) : v;
  }
  return out;
}

export function log(level, event, fields = {}) {
  const entry = {
    ts: new Date().toISOString(),
    level,
    event,
    ...redact(fields),
  };
  buffer.push(entry);
  if (buffer.length > MAX_BUFFER) buffer.shift();
  logBus.emit("entry", entry);
  const line = JSON.stringify(entry);
  if (level === "error") console.error(line);
  else if (level === "warn") console.warn(line);
  else console.log(line);
  if (logFile) {
    try {
      fs.appendFileSync(logFile, line + "\n");
    } catch {
      /* ignore disk errors in logger */
    }
  }
  return entry;
}

export const logger = {
  info: (event, fields) => log("info", event, fields),
  warn: (event, fields) => log("warn", event, fields),
  error: (event, fields) => log("error", event, fields),
  debug: (event, fields) => log("debug", event, fields),
};

export function recentLogs(limit = 200) {
  return buffer.slice(-limit);
}
