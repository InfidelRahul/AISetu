import { CAPABILITIES } from "../domain.js";
import { capabilityReport } from "./profile.js";
import { logger } from "../logging.js";

const REQUIRED_FOR_READY = ["chatInput", "send", "response", "completion"];

export function runValidation(record) {
  const report = capabilityReport(record.profile);
  // Authentication may be user-confirmed
  report.authentication = Boolean(record.authenticated || record.profile.authentication?.ready);
  const failed = REQUIRED_FOR_READY.filter((k) => !report[k]);
  const passed = failed.length === 0;
  record.validation = { passed, report, failed, at: new Date().toISOString() };
  if (passed) {
    record.status = "ready";
    record.profile.ready = true;
  } else {
    record.status = "needs_validation";
    record.profile.ready = false;
  }
  logger.info("validation", { id: record.id, passed, failed });
  return record.validation;
}

export function fullCapabilityTable(report) {
  return CAPABILITIES.map((c) => ({ id: c, ok: Boolean(report[c]) }));
}
