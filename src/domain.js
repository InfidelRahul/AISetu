import { randomUUID } from "node:crypto";

export const MODES = Object.freeze({
  ask: {
    id: "ask",
    name: "Ask",
    instructions:
      "Normal conversation. Keep responses useful and reasonably concise.",
    contextPolicy: "balanced",
    responsePolicy: "concise",
    conversationStrategy: "continue",
  },
  code: {
    id: "code",
    name: "Code",
    instructions:
      "Prioritize implementation-ready code. Code first, minimal explanation, small summary only when useful. Prefer existing functionality. Avoid unnecessary architectural changes.",
    contextPolicy: "code-heavy",
    responsePolicy: "code-first",
    conversationStrategy: "continue",
  },
  debug: {
    id: "debug",
    name: "Debug",
    instructions:
      "Diagnose and fix the problem. Identify likely root cause, use supplied errors/context, provide the concrete fix, avoid irrelevant explanation.",
    contextPolicy: "errors-first",
    responsePolicy: "fix-first",
    conversationStrategy: "continue",
  },
  plan: {
    id: "plan",
    name: "Plan",
    instructions:
      "Provide a concise implementation plan. Short, structured, actionable. Identify important dependencies and risks. Avoid implementation code unless requested.",
    contextPolicy: "overview",
    responsePolicy: "structured",
    conversationStrategy: "continue",
  },
  review: {
    id: "review",
    name: "Review",
    instructions:
      "Review code or architecture. Focus on correctness, maintainability, architecture, concrete issues, and practical improvements. Avoid unnecessary rewrites.",
    contextPolicy: "diff-first",
    responsePolicy: "critique",
    conversationStrategy: "continue",
  },
});

export function newId(prefix = "id") {
  return `${prefix}_${randomUUID().slice(0, 8)}`;
}

export const CAPABILITIES = [
  "authentication",
  "chatInput",
  "send",
  "response",
  "streaming",
  "completion",
  "stop",
  "newChat",
  "history",
  "openConversation",
  "currentConversation",
  "models",
  "modelSwitching",
];

export const PROFILE_VERSION = 1;
