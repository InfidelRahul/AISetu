import fs from "node:fs";
import path from "node:path";
import { AppError, ErrorCode } from "./error.js";

export function atomicWrite(filePath, contents) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const tmp = filePath + ".tmp";
  fs.writeFileSync(tmp, contents);
  fs.renameSync(tmp, filePath);
}

export function readJson(filePath, fallback = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (err) {
    throw new AppError(ErrorCode.PersistenceFailed, `Failed to read ${filePath}`, {
      cause: err.message,
    });
  }
}

export function writeJson(filePath, value) {
  try {
    atomicWrite(filePath, JSON.stringify(value, null, 2));
  } catch (err) {
    throw new AppError(ErrorCode.PersistenceFailed, `Failed to write ${filePath}`, {
      cause: err.message,
    });
  }
}
