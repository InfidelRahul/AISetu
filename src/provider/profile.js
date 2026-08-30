import { CAPABILITIES, PROFILE_VERSION } from "../domain.js";
import { AppError, ErrorCode } from "../error.js";

export function emptyProfile({ name, url }) {
  const caps = {};
  for (const c of CAPABILITIES) caps[c] = { taught: false, valid: false, evidence: null };
  return {
    version: PROFILE_VERSION,
    metadata: { name, createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() },
    website: { url },
    authentication: { required: true, ready: false },
    capabilities: caps,
    interactions: {},
    conversationStrategy: {},
    modelStrategy: { models: [], current: null },
    ready: false,
  };
}

export function migrateProfile(raw) {
  if (!raw || typeof raw !== "object") {
    throw new AppError(ErrorCode.ProfileInvalid, "Profile is not an object");
  }
  const version = raw.version || 0;
  let p = { ...raw };
  if (version < 1) p.version = PROFILE_VERSION;
  if (!p.capabilities) p.capabilities = emptyProfile({ name: "x", url: "" }).capabilities;
  return p;
}

export function validateProfile(profile) {
  const missing = [];
  if (!profile?.website?.url) missing.push("website.url");
  if (!profile?.metadata?.name) missing.push("metadata.name");
  if (missing.length) {
    throw new AppError(ErrorCode.ProfileInvalid, "Profile invalid", { missing });
  }
  return true;
}

export function capabilityReport(profile) {
  const report = {};
  for (const c of CAPABILITIES) {
    const cap = profile.capabilities?.[c];
    report[c] = Boolean(cap?.valid || cap?.taught);
  }
  return report;
}

export function markCapability(profile, key, evidence) {
  if (!profile.capabilities[key]) profile.capabilities[key] = { taught: false, valid: false };
  profile.capabilities[key].taught = true;
  profile.capabilities[key].evidence = evidence || { at: new Date().toISOString() };
  profile.metadata.updatedAt = new Date().toISOString();
  return profile;
}
