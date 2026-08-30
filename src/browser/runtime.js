import { EventEmitter } from "node:events";
import { emptyPage } from "./page_model.js";
import { AppError, ErrorCode } from "../error.js";
import { logger } from "../logging.js";

/**
 * Provider-independent browser runtime.
 * Sessions are isolated per provider.
 */
export class BrowserRuntime extends EventEmitter {
  constructor(providerId) {
    super();
    this.providerId = providerId;
    this.status = "stopped";
    this.page = emptyPage();
    this.url = "";
  }

  async start(url) {
    this.status = "starting";
    this.url = url;
    this.page.url = url;
    this.status = "running";
    logger.info("browser.navigation", { providerId: this.providerId, url });
    this.emit("status", this.status);
    return this.page;
  }

  async stop() {
    this.status = "stopped";
    this.emit("status", this.status);
  }

  async restart(url) {
    await this.stop();
    return this.start(url || this.url);
  }

  async observe() {
    return this.page;
  }

  async typeText(_text) {
    throw new AppError(ErrorCode.BrowserUnavailable, "No live browser backend");
  }

  async send() {
    throw new AppError(ErrorCode.BrowserUnavailable, "No live browser backend");
  }

  async stopGeneration() {
    throw new AppError(ErrorCode.CapabilityNotFound, "Stop not available");
  }

  async newChat() {
    throw new AppError(ErrorCode.CapabilityNotFound, "New chat not available");
  }

  async openConversation(_id) {
    throw new AppError(ErrorCode.ConversationUnavailable, "Open conversation not available");
  }

  async selectModel(_id) {
    throw new AppError(ErrorCode.ModelUnavailable, "Model select not available");
  }
}

/** In-app session: records URL and auth flag the user confirms in the UI iframe. */
export class EmbeddedSessionRuntime extends BrowserRuntime {
  markAuthenticated(yes) {
    this.page.authenticated = Boolean(yes);
    logger.info("browser.auth", { providerId: this.providerId, authenticated: yes });
  }

  applyPage(partial) {
    this.page = { ...this.page, ...partial };
    this.emit("page", this.page);
  }
}
