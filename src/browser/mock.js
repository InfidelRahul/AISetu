import { BrowserRuntime } from "./runtime.js";
import { logger } from "../logging.js";

const MOCK_MODELS = [
  { id: "mock-fast", name: "Mock Fast" },
  { id: "mock-reason", name: "Mock Reason" },
];

/**
 * Deterministic mock browser for tests and for API use before a real provider is taught.
 */
export class MockBrowserRuntime extends BrowserRuntime {
  constructor(providerId) {
    super(providerId);
    this.history = [{ id: "c1", title: "Welcome" }];
    this.currentId = "c1";
    this.modelId = "mock-fast";
    this.input = "";
    this.messages = [];
    this.generating = false;
    this._timer = null;
    this._full = "";
  }

  async start(url) {
    await super.start(url || "mock://provider");
    this.page.authenticated = true;
    this.page.model = { current: this.modelId, available: MOCK_MODELS };
    this.page.conversations = this.history;
    this.page.currentConversation = this.currentId;
    this.page.composer = { input: "textarea", send: "button.send", stop: "button.stop", value: "" };
    return this.page;
  }

  async typeText(text) {
    this.input = text;
    this.page.composer.value = text;
    logger.debug("browser.type", { providerId: this.providerId, n: text.length });
  }

  async send({ onDelta, signal } = {}) {
    this.generating = true;
    this.page.generating = true;
    const prompt = this.input;
    this.messages.push({ role: "user", text: prompt });
    this.input = "";
    const reply = mockReply(prompt);
    this._full = "";
    const chunks = chunkText(reply, 12);
    for (const c of chunks) {
      if (signal?.aborted) {
        this.generating = false;
        this.page.generating = false;
        const err = new Error("aborted");
        err.name = "AbortError";
        throw err;
      }
      this._full += c;
      onDelta?.(c, this._full);
      await delay(18);
    }
    this.messages.push({ role: "assistant", text: this._full });
    this.page.messages = this.messages.slice(-20);
    this.generating = false;
    this.page.generating = false;
    return this._full;
  }

  async stopGeneration() {
    this.generating = false;
    this.page.generating = false;
  }

  async newChat() {
    const id = `c${this.history.length + 1}`;
    this.history.unshift({ id, title: "New chat" });
    this.currentId = id;
    this.messages = [];
    this.page.conversations = this.history;
    this.page.currentConversation = id;
    return id;
  }

  async openConversation(id) {
    const found = this.history.find((c) => c.id === id);
    if (!found) return null;
    this.currentId = id;
    this.page.currentConversation = id;
    return id;
  }

  async selectModel(id) {
    const found = MOCK_MODELS.find((m) => m.id === id);
    if (!found) return false;
    this.modelId = id;
    this.page.model.current = id;
    return true;
  }
}

function mockReply(prompt) {
  const p = prompt.slice(0, 400);
  if (/mode:\s*code/i.test(prompt)) {
    return "```js\nexport function run() {\n  return 42;\n}\n```\n\nSummary: stub implementation matching the request.\n";
  }
  if (/mode:\s*debug/i.test(prompt)) {
    return "Likely cause: unhandled null. Fix: guard the value before use, then re-run the failing path.\n";
  }
  if (/mode:\s*plan/i.test(prompt)) {
    return "1. Confirm inputs\n2. Implement core path\n3. Add tests\nRisks: browser-provider coupling.\n";
  }
  if (/mode:\s*review/i.test(prompt)) {
    return "Correctness: check error paths. Maintainability: keep engines separate from the API layer.\n";
  }
  return `Mock provider received the prompt and replies as a local stand-in until a taught website is ready.\n\n${p}`;
}

function chunkText(text, size) {
  const out = [];
  for (let i = 0; i < text.length; i += size) out.push(text.slice(i, i + size));
  return out.length ? out : [""];
}

function delay(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

export { MOCK_MODELS };
