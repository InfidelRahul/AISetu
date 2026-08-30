import { ProviderRepository } from "./provider/repository.js";
import { ModelRegistry } from "./engine/models.js";
import { markCapability } from "./provider/profile.js";
import { runValidation } from "./provider/validation.js";
import { nextStep, uniqueTestMessage, STEP_COPY, STAGES } from "./teaching/machine.js";
import { EmbeddedSessionRuntime } from "./browser/runtime.js";
import { logger } from "./logging.js";

export function createState(config) {
  const repo = new ProviderRepository(config.dataDir);
  const registry = new ModelRegistry();
  const sessions = {};
  const browsers = {};

  function refreshRegistry() {
    for (const p of repo.list()) {
      const models = p.profile?.modelStrategy?.models || [];
      registry.setModels(p.id, models, p.profile?.modelStrategy?.current);
    }
  }
  refreshRegistry();

  return {
    config,
    repo,
    registry,
    sessions,
    browsers,
    refreshRegistry,

    teachingStart(id) {
      const rec = repo.get(id);
      const first = nextStep(null);
      rec.teaching = {
        stage: first.stage,
        step: first.step,
        testMessage: uniqueTestMessage(),
        startedAt: new Date().toISOString(),
      };
      rec.status = "teaching";
      repo.save(rec);
      logger.info("teaching.start", { id, step: first.step });
      return rec;
    },

    teachingConfirm(id, { notes, models, currentModel }) {
      const rec = repo.get(id);
      if (!rec.teaching?.step) this.teachingStart(id);
      const step = rec.teaching.step;
      const evidence = {
        at: new Date().toISOString(),
        notes: notes || null,
        testMessage: rec.teaching.testMessage,
        before: rec.teaching.step,
      };
      if (step !== "validate") {
        rec.profile = markCapability(rec.profile, step, evidence);
        rec.profile.interactions[step] = evidence;
      }
      if (step === "models" && Array.isArray(models) && models.length) {
        rec.profile.modelStrategy.models = models.map((m) =>
          typeof m === "string" ? { id: slug(m), name: m } : m
        );
        rec.profile.modelStrategy.current = currentModel || rec.profile.modelStrategy.models[0]?.id;
        this.refreshRegistry();
      }
      if (step === "chatInput" || step === "send") {
        rec.profile.authentication.ready = rec.authenticated;
      }
      const nxt = nextStep(step);
      rec.teaching.stage = nxt.stage;
      rec.teaching.step = nxt.step;
      if (nxt.step === "validate" || step === "validate") {
        runValidation(rec);
      }
      repo.save(rec);
      logger.info("teaching.confirm", { id, step, next: rec.teaching.step });
      return rec;
    },

    teachingSkip(id) {
      const rec = repo.get(id);
      const nxt = nextStep(rec.teaching?.step);
      rec.teaching = rec.teaching || {};
      rec.teaching.stage = nxt.stage;
      rec.teaching.step = nxt.step;
      repo.save(rec);
      return rec;
    },

    setAuth(id, authenticated) {
      const rec = repo.update(id, { authenticated: Boolean(authenticated) });
      rec.profile.authentication.ready = Boolean(authenticated);
      if (authenticated) markCapability(rec.profile, "authentication", { at: new Date().toISOString() });
      repo.save(rec);
      const br = browsers[id];
      if (br) br.markAuthenticated(authenticated);
      return rec;
    },

    openBrowser(id) {
      const rec = repo.get(id);
      if (!browsers[id]) browsers[id] = new EmbeddedSessionRuntime(id);
      browsers[id].start(rec.url);
      rec.session = { status: "running", url: rec.url };
      repo.save(rec);
      return rec;
    },

    validate(id) {
      const rec = repo.get(id);
      runValidation(rec);
      repo.save(rec);
      return rec;
    },

    teachingMeta() {
      return { stages: STAGES, copy: STEP_COPY };
    },
  };
}

function slug(s) {
  return String(s)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "model";
}
