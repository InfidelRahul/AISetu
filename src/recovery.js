import { logger } from "./logging.js";
import { AppError, ErrorCode } from "./error.js";

/**
 * Deterministic recovery for failed learned interactions.
 */
export async function recover(runtime, action, args) {
  logger.warn("recovery.attempt", { action });
  try {
    await runtime.observe();
    return await runtime[action](...args);
  } catch (err) {
    logger.error("recovery.failed", { action, message: err.message });
    throw new AppError(ErrorCode.GenerationFailed, `Unrecoverable: ${action}`, {
      retryable: false,
    });
  }
}
