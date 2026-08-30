/** Structured internal errors. Avoid stringly-typed handling. */

export const ErrorCode = Object.freeze({
  ProviderNotConfigured: "ProviderNotConfigured",
  AuthenticationRequired: "AuthenticationRequired",
  BrowserUnavailable: "BrowserUnavailable",
  CapabilityNotFound: "CapabilityNotFound",
  ConversationUnavailable: "ConversationUnavailable",
  ModelUnavailable: "ModelUnavailable",
  GenerationFailed: "GenerationFailed",
  GenerationCancelled: "GenerationCancelled",
  ProviderValidationFailed: "ProviderValidationFailed",
  RequestInvalid: "RequestInvalid",
  PersistenceFailed: "PersistenceFailed",
  ProfileInvalid: "ProfileInvalid",
  ModeUnavailable: "ModeUnavailable",
  Internal: "Internal",
});

export class AppError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "AppError";
    this.code = code;
    this.details = details;
    this.retryable = Boolean(details.retryable);
  }

  toJSON() {
    return {
      error: {
        type: this.code,
        message: this.message,
        details: this.details,
      },
    };
  }

  httpStatus() {
    switch (this.code) {
      case ErrorCode.RequestInvalid:
      case ErrorCode.ProfileInvalid:
        return 400;
      case ErrorCode.AuthenticationRequired:
        return 401;
      case ErrorCode.ProviderNotConfigured:
      case ErrorCode.ModelUnavailable:
      case ErrorCode.ModeUnavailable:
      case ErrorCode.CapabilityNotFound:
      case ErrorCode.ConversationUnavailable:
        return 404;
      case ErrorCode.GenerationCancelled:
        return 499;
      case ErrorCode.ProviderValidationFailed:
      case ErrorCode.GenerationFailed:
        return 422;
      case ErrorCode.BrowserUnavailable:
        return 503;
      default:
        return 500;
    }
  }
}

export function asAppError(err) {
  if (err instanceof AppError) return err;
  if (err?.name === "AbortError") {
    return new AppError(ErrorCode.GenerationCancelled, "Generation cancelled");
  }
  return new AppError(ErrorCode.Internal, err?.message || String(err));
}
