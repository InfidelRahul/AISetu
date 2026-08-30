import { EventEmitter } from "node:events";

/**
 * Response observer: generation start, deltas, completion, failure, cancel.
 */
export class ResponseObserver extends EventEmitter {
  constructor() {
    super();
    this.previous = "";
    this.started = false;
  }

  reset() {
    this.previous = "";
    this.started = false;
  }

  observeText(fullText) {
    const text = fullText ?? "";
    if (!this.started) {
      this.started = true;
      this.emit("event", { type: "GenerationStarted" });
    }
    if (text.startsWith(this.previous)) {
      const delta = text.slice(this.previous.length);
      if (delta) this.emit("event", { type: "TextDelta", text: delta });
    } else if (text.length) {
      this.emit("event", { type: "TextDelta", text, replace: true });
    }
    this.previous = text;
  }

  complete() {
    if (!this.started) {
      this.started = true;
      this.emit("event", { type: "GenerationStarted" });
    }
    this.emit("event", { type: "GenerationCompleted", text: this.previous });
  }

  fail(message) {
    this.emit("event", { type: "GenerationFailed", message });
  }

  cancel() {
    this.emit("event", { type: "GenerationCancelled" });
  }
}

export function computeDelta(previous, next) {
  if (next.startsWith(previous)) return next.slice(previous.length);
  return next;
}
