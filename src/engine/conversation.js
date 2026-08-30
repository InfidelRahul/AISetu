import { logger } from "../logging.js";
import { AppError, ErrorCode } from "../error.js";

export class ConversationEngine {
  constructor(runtime) {
    this.runtime = runtime;
  }

  async current() {
    const page = await this.runtime.observe();
    return page.currentConversation;
  }

  async history() {
    const page = await this.runtime.observe();
    return page.conversations || [];
  }

  async newConversation() {
    const id = await this.runtime.newChat();
    const cur = await this.current();
    if (id && cur && id !== cur) {
      logger.warn("conversation.verify_mismatch", { expected: id, current: cur });
    }
    logger.info("conversation.new", { id });
    return id;
  }

  async open(id) {
    const ok = await this.runtime.openConversation(id);
    if (!ok) throw new AppError(ErrorCode.ConversationUnavailable, `Cannot open ${id}`);
    return ok;
  }
}
