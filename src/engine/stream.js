/**
 * Stream normalizer: internal events → OpenAI-compatible SSE chunks.
 */

export function openaiChunk({ id, model, delta, finish = null }) {
  return {
    id,
    object: "chat.completion.chunk",
    created: Math.floor(Date.now() / 1000),
    model,
    choices: [
      {
        index: 0,
        delta: delta || {},
        finish_reason: finish,
      },
    ],
  };
}

export function openaiCompletion({ id, model, content, finish = "stop" }) {
  return {
    id,
    object: "chat.completion",
    created: Math.floor(Date.now() / 1000),
    model,
    choices: [
      {
        index: 0,
        message: { role: "assistant", content },
        finish_reason: finish,
      },
    ],
    usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
  };
}

export function sse(data) {
  return `data: ${JSON.stringify(data)}\n\n`;
}

export function sseDone() {
  return "data: [DONE]\n\n";
}
