import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { AppError, asAppError, ErrorCode } from "../error.js";
import { logger, recentLogs, logBus } from "../logging.js";
import { handleChatCompletions } from "../engine/pipeline.js";
import { listModes } from "../engine/modes.js";
import { saveConfig } from "../config.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_DIR = path.join(__dirname, "../../ui");

export function createServer(state) {
  const server = http.createServer(async (req, res) => {
    try {
      await route(state, req, res);
    } catch (err) {
      const app = asAppError(err);
      logger.error("http.error", { code: app.code, message: app.message, path: req.url });
      if (!res.headersSent) {
        json(res, app.httpStatus(), app.toJSON());
      }
    }
  });
  return server;
}

async function route(state, req, res) {
  const url = new URL(req.url, "http://localhost");
  const p = url.pathname;
  cors(res);
  if (req.method === "OPTIONS") {
    res.writeHead(204);
    res.end();
    return;
  }

  if (p === "/v1/models" && req.method === "GET") {
    state.refreshRegistry();
    const data = state.registry.listAll(state.repo.list());
    json(res, 200, { object: "list", data });
    return;
  }

  if (p === "/v1/chat/completions" && req.method === "POST") {
    const body = await readJson(req);
    const stream = Boolean(body.stream);
    const ac = new AbortController();
    req.on("close", () => ac.abort());
    if (stream) {
      res.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
        "Access-Control-Allow-Origin": "*",
      });
      await handleChatCompletions({
        body,
        signal: ac.signal,
        registry: state.registry,
        providers: state.repo,
        sessions: state.sessions,
        stream: true,
        onSse: (chunk) => res.write(chunk),
      });
      res.end();
    } else {
      const out = await handleChatCompletions({
        body,
        signal: ac.signal,
        registry: state.registry,
        providers: state.repo,
        sessions: state.sessions,
        stream: false,
      });
      json(res, 200, out);
    }
    return;
  }

  if (p === "/api/health") {
    json(res, 200, { ok: true, name: "AISetu" });
    return;
  }

  if (p === "/api/status") {
    json(res, 200, {
      config: { port: state.config.port, dataDir: state.config.dataDir },
      providers: state.repo.list().map(summary),
      modes: listModes(),
    });
    return;
  }

  if (p === "/api/config" && req.method === "GET") {
    json(res, 200, state.config);
    return;
  }
  if (p === "/api/config" && req.method === "PUT") {
    const body = await readJson(req);
    Object.assign(state.config, body);
    saveConfig(state.config);
    json(res, 200, state.config);
    return;
  }

  if (p === "/api/logs" && req.method === "GET") {
    json(res, 200, { entries: recentLogs(500) });
    return;
  }

  if (p === "/api/modes") {
    json(res, 200, { modes: listModes() });
    return;
  }

  if (p === "/api/teaching/meta") {
    json(res, 200, state.teachingMeta());
    return;
  }

  if (p === "/api/providers" && req.method === "GET") {
    json(res, 200, { providers: state.repo.list() });
    return;
  }

  if (p === "/api/providers" && req.method === "POST") {
    const body = await readJson(req);
    if (!body?.name || !body?.url) {
      throw new AppError(ErrorCode.RequestInvalid, "name and url required");
    }
    const rec = state.repo.create({ name: body.name, url: normalizeUrl(body.url) });
    json(res, 201, rec);
    return;
  }

  const prov = p.match(/^\/api\/providers\/([^/]+)(?:\/(.*))?$/);
  if (prov) {
    const id = decodeURIComponent(prov[1]);
    const rest = prov[2] || "";
    const rec = state.repo.get(id);
    if (!rec && req.method !== "DELETE") {
      throw new AppError(ErrorCode.ProviderNotConfigured, "Provider not found");
    }
    if (req.method === "GET" && !rest) {
      json(res, 200, rec);
      return;
    }
    if (req.method === "DELETE" && !rest) {
      state.repo.remove(id);
      json(res, 200, { ok: true });
      return;
    }
    if (req.method === "PATCH" && !rest) {
      const body = await readJson(req);
      json(res, 200, state.repo.update(id, body));
      return;
    }
    if (rest === "auth" && req.method === "POST") {
      const body = await readJson(req);
      json(res, 200, state.setAuth(id, body.authenticated));
      return;
    }
    if (rest === "browser" && req.method === "POST") {
      json(res, 200, state.openBrowser(id));
      return;
    }
    if (rest === "teach/start" && req.method === "POST") {
      json(res, 200, state.teachingStart(id));
      return;
    }
    if (rest === "teach/confirm" && req.method === "POST") {
      const body = await readJson(req);
      json(res, 200, state.teachingConfirm(id, body || {}));
      return;
    }
    if (rest === "teach/skip" && req.method === "POST") {
      json(res, 200, state.teachingSkip(id));
      return;
    }
    if (rest === "validate" && req.method === "POST") {
      json(res, 200, state.validate(id));
      return;
    }
    if (rest === "models" && req.method === "PUT") {
      const body = await readJson(req);
      rec.profile.modelStrategy.models = body.models || [];
      rec.profile.modelStrategy.current = body.current || rec.profile.modelStrategy.current;
      state.repo.save(rec);
      state.refreshRegistry();
      json(res, 200, rec);
      return;
    }
  }

  if (p.startsWith("/api/")) {
    json(res, 404, { error: { type: "NotFound", message: p } });
    return;
  }

  serveStatic(res, p);
}

function summary(p) {
  return {
    id: p.id,
    name: p.name,
    url: p.url,
    status: p.status,
    authenticated: p.authenticated,
    teaching: p.teaching,
    validation: p.validation,
  };
}

function normalizeUrl(u) {
  if (!/^https?:\/\//i.test(u)) return "https://" + u;
  return u;
}

function cors(res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,PUT,PATCH,DELETE,OPTIONS");
}

function json(res, status, obj) {
  const body = JSON.stringify(obj);
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Access-Control-Allow-Origin": "*",
  });
  res.end(body);
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      const raw = Buffer.concat(chunks).toString("utf8");
      if (!raw) return resolve({});
      try {
        resolve(JSON.parse(raw));
      } catch {
        reject(new AppError(ErrorCode.RequestInvalid, "Invalid JSON"));
      }
    });
    req.on("error", reject);
  });
}

function serveStatic(res, pathname) {
  let rel = pathname === "/" ? "/index.html" : pathname;
  const file = path.normalize(path.join(UI_DIR, rel));
  if (!file.startsWith(UI_DIR)) {
    res.writeHead(403);
    res.end();
    return;
  }
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    const idx = path.join(UI_DIR, "index.html");
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    res.end(fs.readFileSync(idx));
    return;
  }
  const ext = path.extname(file);
  const types = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".svg": "image/svg+xml",
    ".json": "application/json",
    ".png": "image/png",
    ".woff": "font/woff",
    ".woff2": "font/woff2",
  };
  res.writeHead(200, { "Content-Type": types[ext] || "application/octet-stream" });
  res.end(fs.readFileSync(file));
}

void logBus;
