import { logger } from "../logging.js";

/**
 * Normalized model registry: provider → models. No hard-coded vendor lists.
 */
export class ModelRegistry {
  constructor() {
    this.byProvider = new Map();
  }

  setModels(providerId, models, current = null) {
    const list = (models || []).map((m) => ({
      id: m.id,
      name: m.name || m.id,
      owned_by: providerId,
    }));
    this.byProvider.set(providerId, { models: list, current });
    logger.info("models.refresh", { providerId, count: list.length });
    return list;
  }

  listAll(providers) {
    const out = [];
    for (const p of providers) {
      const pack = this.byProvider.get(p.id);
      const models =
        pack?.models?.length
          ? pack.models
          : [{ id: "default", name: "default", owned_by: p.id }];
      for (const m of models) {
        out.push({
          id: `${p.id}/${m.id}`,
          object: "model",
          created: 0,
          owned_by: p.name || p.id,
          permission: [],
        });
      }
    }
    out.push({
      id: "mock/mock-fast",
      object: "model",
      created: 0,
      owned_by: "mock",
      permission: [],
    });
    out.push({
      id: "mock/mock-reason",
      object: "model",
      created: 0,
      owned_by: "mock",
      permission: [],
    });
    return out;
  }

  parse(modelString) {
    const raw = modelString || "mock/mock-fast";
    const i = raw.indexOf("/");
    if (i === -1) return { providerId: "mock", modelId: raw };
    return { providerId: raw.slice(0, i), modelId: raw.slice(i + 1) };
  }
}
