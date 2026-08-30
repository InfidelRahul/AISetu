import { Platform } from "react-native";

const KEY = "aisetu.apiBase";

export function defaultBase() {
  if (Platform.OS === "web") return "";
  if (Platform.OS === "android") return "http://10.0.2.2:8787";
  return "http://127.0.0.1:8787";
}

export function getBase() {
  try {
    const s = globalThis.localStorage?.getItem(KEY);
    if (s != null) return String(s).replace(/\/$/, "");
  } catch {
    /* native may not have localStorage */
  }
  return defaultBase();
}

export function setBase(value) {
  const v = String(value || "").replace(/\/$/, "");
  try {
    globalThis.localStorage?.setItem(KEY, v);
  } catch {
    /* ignore */
  }
}

export const platformLabel = `${Platform.OS}${Platform.Version ? " " + Platform.Version : ""}`;
