import { describe, it } from "node:test";
import assert from "node:assert/strict";
import os from "node:os";
import path from "node:path";
import fs from "node:fs";
import { computeDelta } from "../src/engine/observer.js";
import { composePrompt } from "../src/engine/prompt.js";
import { buildContext } from "../src/engine/context.js";
import { resolveMode } from "../src/engine/modes.js";
import { emptyProfile, migrateProfile, markCapability, capabilityReport } from "../src/provider/profile.js";
import { ProviderRepository } from "../src/provider/repository.js";
import { nextStep, uniqueTestMessage } from "../src/teaching/machine.js";
import { MockBrowserRuntime } from "../src/browser/mock.js";
import { ModelRegistry } from "../src/engine/models.js";
import { AppError, ErrorCode } from "../src/error.js";
import { openaiChunk } from "../src/engine/stream.js";
import { runValidation } from "../src/provider/validation.js";

describe("delta extraction", () => {
  it("appends suffix", () => {
    assert.equal(computeDelta("Hel", "Hello"), "lo");
  });
});

describe("prompt composition", () => {
  it("includes mode and user request", () => {
    const mode = resolveMode("code");
    const ctx = buildContext(mode, { currentFile: "a.js", userRequest: "x" });
    const p = composePrompt({ mode, userRequest: "add fn", contextParts: ctx });
    assert.match(p, /Mode: Code/);
    assert.match(p, /add fn/);
    assert.match(p, /a\.js/);
  });
});

describe("profiles", () => {
  it("round-trips via repository", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "aisetu-"));
    const repo = new ProviderRepository(dir);
    const rec = repo.create({ name: "Demo", url: "https://example.com" });
    assert.equal(repo.get(rec.id).name, "Demo");
    rec.profile = markCapability(rec.profile, "send", { at: "t" });
    repo.save(rec);
    assert.equal(capabilityReport(repo.get(rec.id).profile).send, true);
    migrateProfile(rec.profile);
  });
});

describe("teaching machine", () => {
  it("advances steps", () => {
    const a = nextStep(null);
    assert.equal(a.step, "chatInput");
    const b = nextStep("chatInput");
    assert.equal(b.step, "send");
    assert.match(uniqueTestMessage(), /AISETU_TEACH_/);
  });
});

describe("mock browser streaming", () => {
  it("streams text", async () => {
    const rt = new MockBrowserRuntime("mock");
    await rt.start();
    await rt.typeText("hello");
    let n = 0;
    const full = await rt.send({ onDelta: () => n++ });
    assert.ok(full.length > 0);
    assert.ok(n > 0);
  });
});

describe("model registry", () => {
  it("parses provider/model", () => {
    const r = new ModelRegistry();
    assert.deepEqual(r.parse("prv_1/qwen"), { providerId: "prv_1", modelId: "qwen" });
  });
});

describe("errors", () => {
  it("maps http status", () => {
    const e = new AppError(ErrorCode.ProviderNotConfigured, "x");
    assert.equal(e.httpStatus(), 404);
  });
});

describe("openai chunk", () => {
  it("has choices", () => {
    const c = openaiChunk({ id: "1", model: "m", delta: { content: "a" } });
    assert.equal(c.choices[0].delta.content, "a");
  });
});

describe("validation", () => {
  it("not ready without teaching", () => {
    const rec = {
      id: "p",
      authenticated: false,
      profile: emptyProfile({ name: "n", url: "https://x" }),
    };
    const v = runValidation(rec);
    assert.equal(v.passed, false);
  });
});
