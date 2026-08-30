/** Guided teaching state machine: Messaging → Conversations → Models → Validation */

export const STAGES = [
  {
    id: "messaging",
    title: "Messaging",
    steps: ["chatInput", "send", "response", "streaming", "completion", "stop"],
  },
  {
    id: "conversations",
    title: "Conversations",
    steps: ["newChat", "history", "openConversation", "currentConversation"],
  },
  {
    id: "models",
    title: "Models",
    steps: ["models", "modelSwitching"],
  },
  {
    id: "validation",
    title: "Validation",
    steps: ["validate"],
  },
];

export const STEP_COPY = {
  chatInput: {
    title: "Chat input",
    body: "Focus the chat box on the website, then confirm. AISetu records that a visible composer exists. A unique test phrase will be suggested.",
  },
  send: {
    title: "Send",
    body: "Submit the test message the way you normally would (button or Enter). Confirm after the message is sent.",
  },
  response: {
    title: "Response",
    body: "Wait until an assistant reply is visible and distinct from your message, then confirm.",
  },
  streaming: {
    title: "Streaming",
    body: "Watch the reply grow. Confirm if text appeared incrementally rather than all at once.",
  },
  completion: {
    title: "Completion",
    body: "Confirm when generation has clearly finished (stop control gone, cursor idle, or complete message).",
  },
  stop: {
    title: "Stop generation",
    body: "Start a reply and use the site’s stop control, then confirm. Required for API cancellation.",
  },
  newChat: {
    title: "New conversation",
    body: "Create a new chat on the site and confirm it became the active conversation.",
  },
  history: {
    title: "Conversation history",
    body: "Show that previous conversations are listed in the visible UI.",
  },
  openConversation: {
    title: "Open conversation",
    body: "Open a previous conversation and confirm it loaded.",
  },
  currentConversation: {
    title: "Current conversation",
    body: "Confirm how the active conversation is indicated in the visible UI.",
  },
  models: {
    title: "Models",
    body: "Open the model selector and list visible models. Mark the current model.",
  },
  modelSwitching: {
    title: "Model switching",
    body: "Select another model and confirm the visible current model changed.",
  },
  validate: {
    title: "Validate",
    body: "Run the capability report. The provider is ready only if required messaging capabilities are taught.",
  },
};

export function flattenSteps() {
  const out = [];
  for (const stage of STAGES) {
    for (const step of stage.steps) out.push({ stage: stage.id, step });
  }
  return out;
}

export function nextStep(current) {
  const all = flattenSteps();
  if (!current) return all[0];
  const i = all.findIndex((s) => s.step === current);
  return all[Math.min(i + 1, all.length - 1)];
}

export function uniqueTestMessage() {
  const n = Math.random().toString(36).slice(2, 8);
  return `AISETU_TEACH_${n}_${Date.now().toString(36)}`;
}
