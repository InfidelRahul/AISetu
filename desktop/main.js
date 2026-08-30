import { app, BrowserWindow, Tray, Menu, nativeImage, shell } from "electron";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { loadConfig } from "../src/config.js";
import { initLogging, logger } from "../src/logging.js";
import { createState } from "../src/app_state.js";
import { createServer } from "../src/api/http.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

let win = null;
let tray = null;
let server = null;

async function boot() {
  const config = loadConfig();
  config.host = process.env.AISETU_HOST || "0.0.0.0";
  initLogging(config.logDir);
  const state = createState(config);
  server = createServer(state);
  await new Promise((resolve) => server.listen(config.port, config.host, resolve));
  logger.info("desktop.listen", { port: config.port });
  return `http://127.0.0.1:${config.port}/`;
}

function createWindow(url) {
  win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 900,
    minHeight: 600,
    title: "AISetu",
    backgroundColor: "#f4efe4",
    autoHideMenuBar: true,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  win.loadURL(url);
  win.on("closed", () => {
    win = null;
  });
}

app.whenReady().then(async () => {
  const url = await boot();
  createWindow(url);
  const icon = nativeImage.createEmpty();
  tray = new Tray(icon);
  tray.setToolTip("AISetu");
  tray.setContextMenu(
    Menu.buildFromTemplate([
      { label: "Show AISetu", click: () => win?.show() },
      { label: "Quit", click: () => app.quit() },
    ])
  );
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});

app.on("before-quit", () => {
  try {
    server?.close();
  } catch {
    /* ignore */
  }
});

app.on("web-contents-created", (_e, contents) => {
  contents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: "deny" };
  });
});

void path;
void __dirname;
