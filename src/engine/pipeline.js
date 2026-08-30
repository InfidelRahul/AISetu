import { resolveMode } from "./modes.js";
import { buildContext } from "./context.js";
import { composePrompt } from "./prompt.js";
import { openaiChunk, openaiCompletion, sse, sseDone } from "./stream.js";
import { ResponseObserver } from "./observer.js";
import { MockBrowserRuntime } from "../browser/mock.js";
import { ConversationEngine } from "./conversation.js";
import { AppError, ErrorCode, asAppError } from "../error.js";
import { logger } from "../logging.js";
import { newId } from "../domain.js";

/**
 * HTTP → validate → normalize → resolve provider/model/mode →
 * context → prompt → conversation → browser → observer → OpenAI stream
 */
export async function handleChatCompletions({
  body,
  signal,
  registry,
  providers,
  sessions,
  onSse,
  stream,
}) {
  validateRequest(body);
  const { providerId, modelId } = registry.parse(body.model);
  const modeId = extractMode(body) || "ask";
  const mode = resolveMode(modeId);
  logger.info("api.chat", { providerId, modelId, mode: mode.id, stream: Boolean(stream) });

  const userRequest = lastUserContent(body.messages);
  const contextParts = buildContext(mode, body.context || extractContext(body));
  const prompt = composePrompt({
    mode,
    userRequest,
    contextParts,
    conversationHint: body.conversation,
  });

  const runtime = await resolveRuntime({ providerId, providers, sessions });
  const conv = new ConversationEngine(runtime);
  if (body.new_conversation) await conv.newConversation();
  else if (body.conversation) {
    try {
      await conv.open(body.conversation);
    } catch {
      /* optional */
    }
  }

  if (modelId && runtime.selectModel) {
    try {
      await runtime.selectModel(modelId);
    } catch {
      /* mock / untaught */
    }
  }

  await runtime.typeText(prompt);
  const completionId = `chatcmpl_${newId("x")}`;
  const modelName = `${providerId}/${modelId}`;
  const observer = new ResponseObserver();

  if (stream && onSse) {
    onSse(sse(openaiChunk({ id: completionId, model: modelName, delta: { role: "assistant" } })));
    observer.on("event", (ev) => {
      if (ev.type === "TextDelta" && ev.text) {
        onSse(sse(openaiChunk({ id: completionId, model: modelName, delta: { content: ev.text } })));
      }
    });
  }

  let full = "";
  try {
    full = await runtime.send({
      signal,
      onDelta: (delta, acc) => {
        observer.observeText(acc);
        full = acc;
      },
    });
    observer.complete();
  } catch (err) {
    const app = asAppError(err);
    if (app.code === ErrorCode.GenerationCancelled) {
      observer.cancel();
      if (runtime.stopGeneration) await runtime.stopGeneration().catch(() => {});
      if (stream && onSse) {
        onSse(sse(openaiChunk({ id: completionId, model: modelName, delta: {}, finish: "stop" })));
        onSse(sseDone());
      }
      throw app;
    }
    observer.fail(app.message);
    throw app;
  }

  if (stream && onSse) {
    onSse(sse(openaiChunk({ id: completionId, model: modelName, delta: {}, finish: "stop" })));
    onSse(sseDone());
    return { streamed: true, id: completionId, content: full };
  }
  return openaiCompletion({ id: completionId, model: modelName, content: full });
}

function validateRequest(body) {
  if (!body || typeof body !== "object") {
    throw new AppError(ErrorCode.RequestInvalid, "JSON body required");
  }
  if (!Array.isArray(body.messages) || body.messages.length === 0) {
    throw new AppError(ErrorCode.RequestInvalid, "messages[] required");
  }
}

function lastUserContent(messages) {
  const users = messages.filter((m) => m.role === "user");
  const last = users[users.length - 1] || messages[messages.length - 1];
  const c = last?.content;
  if (typeof c === "string") return c;
  if (Array.isArray(c)) return c.map((p) => p.text || "").join("\n");
  return String(c || "");
}

function extractMode(body) {
  if (body.mode) return body.mode;
  const sys = body.messages.find((m) => m.role === "system");
  const t = typeof sys?.content === "string" ? sys.content : "";
  const m = t.match(/\bmode\s*[:=]\s*(ask|code|debug|plan|review)/i);
  return m ? m[1].toLowerCase() : null;
}

function extractContext(body) {
  const sys = body.messages.filter((m) => m.role === "system").map((m) => m.content).join("\n");
  return { userRequest: "", project: sys || undefined };
}

async function resolveRuntime({ providerId, providers, sessions }) {
  if (providerId === "mock") {
    if (!sessions.mock) {
      sessions.mock = new MockBrowserRuntime("mock");
      await sessions.mock.start("mock://local");
    }
    return sessions.mock;
  }
  const rec = providers.get(providerId);
  if (!rec) throw new AppError(ErrorCode.ProviderNotConfigured, `Provider ${providerId} not found`);
  if (!sessions[providerId]) {
    const rt = new MockBrowserRuntime(providerId);
    await rt.start(rec.url);
    rt.page.authenticated = rec.authenticated;
    if (rec.profile?.modelStrategy?.models?.length) {
      rt.page.model.available = rec.profile.modelStrategy.models;
    }
    sessions[providerId] = rt;
  }
  return sessions[providerId];
}
