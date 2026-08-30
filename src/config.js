import fs from "node:fs";
import path from "node:path";
import os from "node:os";

export function defaultDataDir() {
  if (process.env.AISETU_DATA) return process.env.AISETU_DATA;
  return path.join(os.homedir(), ".aisetu");
}

export function loadConfig(dataDir = defaultDataDir()) {
  fs.mkdirSync(dataDir, { recursive: true });
  const file = path.join(dataDir, "config.json");
  const defaults = {
    host: process.env.AISETU_HOST || "0.0.0.0",
    port: Number(process.env.AISETU_PORT || 8787),
    apiBind: "127.0.0.1",
    logLevel: "info",
    defaultMode: "ask",
    mockProviderEnabled: true,
  };
  let stored = {};
  if (fs.existsSync(file)) {
    try {
      stored = JSON.parse(fs.readFileSync(file, "utf8"));
    } catch {
      stored = {};
    }
  }
  const config = { ...defaults, ...stored, dataDir, logDir: path.join(dataDir, "logs") };
  fs.mkdirSync(config.logDir, { recursive: true });
  fs.writeFileSync(file, JSON.stringify(config, null, 2));
  return config;
}

export function saveConfig(config) {
  const file = path.join(config.dataDir, "config.json");
  fs.writeFileSync(file, JSON.stringify(config, null, 2));
}
