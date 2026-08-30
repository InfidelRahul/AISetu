import fs from "node:fs";
import path from "node:path";
import { newId } from "../domain.js";
import { readJson, writeJson } from "../persist.js";
import { emptyProfile, migrateProfile, validateProfile } from "./profile.js";
import { AppError, ErrorCode } from "../error.js";
import { logger } from "../logging.js";

export class ProviderRepository {
  constructor(dataDir) {
    this.dir = path.join(dataDir, "providers");
    fs.mkdirSync(this.dir, { recursive: true });
  }

  list() {
    return fs
      .readdirSync(this.dir)
      .filter((f) => f.endsWith(".json"))
      .map((f) => this.get(f.replace(/\.json$/, "")))
      .filter(Boolean);
  }

  get(id) {
    const raw = readJson(path.join(this.dir, `${id}.json`), null);
    if (!raw) return null;
    if (raw.profile) raw.profile = migrateProfile(raw.profile);
    return raw;
  }

  create({ name, url }) {
    const id = newId("prv");
    const profile = emptyProfile({ name, url });
    const record = {
      id,
      name,
      url,
      status: "draft",
      authenticated: false,
      teaching: { stage: "idle", step: null },
      validation: { passed: false, report: {} },
      session: { status: "stopped" },
      profile,
      createdAt: new Date().toISOString(),
    };
    this.save(record);
    logger.info("provider.created", { id, name, url });
    return record;
  }

  save(record) {
    record.updatedAt = new Date().toISOString();
    writeJson(path.join(this.dir, `${record.id}.json`), record);
    return record;
  }

  update(id, patch) {
    const rec = this.get(id);
    if (!rec) throw new AppError(ErrorCode.ProviderNotConfigured, `Unknown provider ${id}`);
    Object.assign(rec, patch);
    if (patch.profile) {
      rec.profile = migrateProfile({ ...rec.profile, ...patch.profile });
      validateProfile(rec.profile);
    }
    return this.save(rec);
  }

  remove(id) {
    const file = path.join(this.dir, `${id}.json`);
    if (fs.existsSync(file)) fs.unlinkSync(file);
    logger.info("provider.removed", { id });
  }
}
