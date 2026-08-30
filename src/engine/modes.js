import { MODES } from "../domain.js";
import { AppError, ErrorCode } from "../error.js";

export function resolveMode(id) {
  const key = (id || "ask").toLowerCase();
  const mode = MODES[key];
  if (!mode) {
    throw new AppError(ErrorCode.ModeUnavailable, `Unknown mode: ${id}`);
  }
  return mode;
}

export function listModes() {
  return Object.values(MODES);
}
