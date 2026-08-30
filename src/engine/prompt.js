import { formatContext } from "./context.js";

/**
 * Prompt Composer — independent from browser implementation.
 * Mode instructions + user request + relevant context + conversation state.
 */
export function composePrompt({ mode, userRequest, contextParts, conversationHint }) {
  const sections = [];
  sections.push(`Mode: ${mode.name}\n${mode.instructions}`);
  const ctx = formatContext(contextParts || []);
  if (ctx) sections.push(`Context:\n${ctx}`);
  if (conversationHint) sections.push(`Conversation: ${conversationHint}`);
  sections.push(`User request:\n${userRequest}`);
  return sections.join("\n\n");
}
