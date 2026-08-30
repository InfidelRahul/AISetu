const NAV = [
  ["providers", "Providers"],
  ["browser", "Browser"],
  ["teaching", "Teaching"],
  ["chat", "Chat"],
  ["models", "Models"],
  ["conversations", "Conversations"],
  ["modes", "Modes"],
  ["settings", "Settings"],
  ["logs", "Logs"],
];

const state = {
  view: "providers",
  providers: [],
  selected: null,
  modes: [],
  logs: [],
  meta: { copy: {}, stages: [] },
  status: null,
  chat: { messages: [], streaming: false, model: "mock/mock-fast", mode: "ask" },
};

async function api(path, opts = {}) {
  const res = await fetch(path, {
    headers: { "Content-Type": "application/json", ...(opts.headers || {}) },
    ...opts,
    body: opts.body && typeof opts.body !== "string" ? JSON.stringify(opts.body) : opts.body,
  });
  const ct = res.headers.get("content-type") || "";
  if (ct.includes("text/event-stream")) return res;
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error?.message || res.statusText);
  return data;
}

async function refresh() {
  const [plist, modes, status, meta] = await Promise.all([
    api("/api/providers"),
    api("/api/modes"),
    api("/api/status"),
    api("/api/teaching/meta"),
  ]);
  state.providers = plist.providers;
  state.modes = modes.modes;
  state.status = status;
  state.meta = meta;
  if (state.selected) {
    state.selected = state.providers.find((p) => p.id === state.selected.id) || state.selected;
  }
}

function el(html) {
  const t = document.createElement("template");
  t.innerHTML = html.trim();
  return t.content;
}

function pill(status) {
  const cls = status === "ready" ? "ok" : status === "teaching" ? "warn" : "";
  return `<span class="pill ${cls}">${status || "draft"}</span>`;
}

function frame(content) {
  return `
    <div class="app">
      <header class="titlebar">
        <span class="dots"><i class="r"></i><i class="y"></i><i class="g"></i></span>
        <span class="name">AISetu</span>
        <span class="meta" style="color:#cbbba6">local gateway</span>
      </header>
      <div class="workspace">
        <nav class="side">
          <p class="brand">AISetu</p>
          <p class="tag">Web chat, as an API</p>
          ${NAV.map(
            ([id, label]) =>
              `<button data-nav="${id}" class="${state.view === id ? "active" : ""}">${label}</button>`
          ).join("")}
          <div class="grow"></div>
          <div class="api-chip">Clients use this app as http://127.0.0.1:${state.status?.config?.port || 8787}/v1 — no terminal required.</div>
        </nav>
        <main class="page">${content}</main>
      </div>
    </div>
  `;
}

function render() {
  const root = document.getElementById("app");
  const views = {
    providers: viewProviders,
    browser: viewBrowser,
    teaching: viewTeaching,
    chat: viewChat,
    models: viewModels,
    conversations: viewConversations,
    modes: viewModes,
    settings: viewSettings,
    logs: viewLogs,
  };
  root.innerHTML = "";
  root.append(el(frame((views[state.view] || viewProviders)())));
  bind();
}

function viewProviders() {
  const cards = state.providers
    .map(
      (p) => `
      <article class="card" data-select="${p.id}">
        <h3>${esc(p.name)}</h3>
        <p class="meta">${esc(p.url)}</p>
        <p>${pill(p.status)} ${p.authenticated ? '<span class="pill ok">signed in</span>' : '<span class="pill">auth unknown</span>'}</p>
        <div class="row" style="margin-top:8px">
          <button class="primary" data-open="${p.id}">Open in app</button>
          <button class="danger" data-del="${p.id}">Remove</button>
        </div>
      </article>`
    )
    .join("");
  return `
    <h1>Providers</h1>
    <p class="sub">Add a website, sign in inside the app, teach the chat UI, then talk to it from this window or any IDE.</p>
    <div class="card" style="margin-bottom:12px">
      <div class="row">
        <div style="flex:1;min-width:140px"><label>Name</label><input id="pname" placeholder="Work chat" /></div>
        <div style="flex:2;min-width:200px"><label>Website URL</label><input id="purl" placeholder="https://chat.example.com" /></div>
        <div><label>&nbsp;</label><button class="primary" id="addp">Add</button></div>
      </div>
    </div>
    <div class="grid">${cards || "<p class='meta'>No providers yet. Add one, or use Chat with the built-in mock model.</p>"}</div>
  `;
}

function viewBrowser() {
  const p = state.selected;
  if (!p) return `<h1>Browser</h1><p class="sub">Select a provider on the Providers screen.</p>`;
  return `
    <h1>${esc(p.name)}</h1>
    <p class="sub">Sign in as you normally would. If the site blocks embedding, it may stay blank — mark signed-in after you log in in your usual browser.</p>
    <div class="browser-frame">
      <div class="browser-bar">
        <input id="burl" value="${esc(p.url)}" />
        <button class="primary" id="go">Load</button>
        <button id="authyes">Signed in</button>
        <button id="authno">Signed out</button>
      </div>
      <iframe id="frame" src="${esc(p.url)}" sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-downloads"></iframe>
    </div>
  `;
}

function viewTeaching() {
  const p = state.selected;
  if (!p) return `<h1>Teaching</h1><p class="sub">Select a provider first.</p>`;
  const step = p.teaching?.step;
  const copy = state.meta.copy?.[step] || {
    title: "Start teaching",
    body: "Walk through how this site’s visible chat works. Confirm each step after you do it in the Browser view.",
  };
  const report = p.validation?.report || {};
  const rows = Object.entries(report)
    .map(([k, v]) => `<tr><td>${k}</td><td>${v ? "✓" : "—"}</td></tr>`)
    .join("");
  return `
    <h1>Teaching</h1>
    <p class="sub">${esc(p.name)} · ${esc(p.teaching?.stage || "idle")} · ${esc(step || "not started")}</p>
    <div class="split">
      <div class="card">
        <h3>${esc(copy.title)}</h3>
        <p>${esc(copy.body)}</p>
        ${step === "chatInput" ? `<p>Test phrase: <span class="kbd">${esc(p.teaching?.testMessage || "")}</span></p>` : ""}
        ${step === "models" ? `<label>Visible models</label><input id="modelnames" placeholder="Qwen3-Coder, Fast" />` : ""}
        <label>Notes</label>
        <textarea id="tnotes" rows="2"></textarea>
        <div class="row" style="margin-top:10px">
          <button class="primary" id="tstart">Start</button>
          <button class="primary" id="tconfirm">Confirm</button>
          <button id="tskip">Skip</button>
          <button id="tval">Validate</button>
        </div>
      </div>
      <div class="card">
        <h3>Capabilities</h3>
        <table class="cap-table">${rows || "<tr><td>Not validated</td></tr>"}</table>
        <p>${p.validation?.passed ? '<span class="pill ok">ready</span>' : '<span class="pill warn">not ready</span>'}</p>
      </div>
    </div>
  `;
}

function viewChat() {
  const models = ["mock/mock-fast", "mock/mock-reason"];
  for (const p of state.providers) {
    for (const m of p.profile?.modelStrategy?.models || [{ id: "default", name: "default" }]) {
      models.push(`${p.id}/${m.id}`);
    }
  }
  const bubbles = state.chat.messages
    .map((m) => `<div class="bubble ${m.role}">${esc(m.content)}</div>`)
    .join("");
  return `
    <h1>Chat</h1>
    <p class="sub">Talk through AISetu without an IDE. Streaming is shown live in this window.</p>
    <div class="row" style="margin-bottom:10px">
      <div style="min-width:220px"><label>Model</label>
        <select id="cmodel">${models.map((m) => `<option ${m === state.chat.model ? "selected" : ""}>${m}</option>`).join("")}</select>
      </div>
      <div style="min-width:140px"><label>Mode</label>
        <select id="cmode">${["ask", "code", "debug", "plan", "review"]
          .map((m) => `<option ${m === state.chat.mode ? "selected" : ""}>${m}</option>`)
          .join("")}</select>
      </div>
    </div>
    <div class="chat">
      <div class="transcript" id="transcript">${bubbles || '<p class="meta">Send a message to the mock provider or a taught site.</p>'}</div>
      <div class="composer">
        <textarea id="cinput" rows="2" placeholder="Message"></textarea>
        <button class="primary" id="csend">Send</button>
      </div>
    </div>
  `;
}

function viewModels() {
  const p = state.selected;
  const models = p?.profile?.modelStrategy?.models || [];
  return `
    <h1>Models</h1>
    <p class="sub">From teaching — not a hard-coded vendor list.</p>
    ${
      p
        ? `<div class="card"><h3>${esc(p.name)}</h3>
        <ul>${models.map((m) => `<li>${esc(p.id)}/${esc(m.id)} — ${esc(m.name)}</li>`).join("") || "<li>None yet</li>"}</ul></div>`
        : "<p class='meta'>Select a provider, or use mock/mock-fast in Chat.</p>"
    }
  `;
}

function viewConversations() {
  return `
    <h1>Conversations</h1>
    <p class="sub">New chat, history, open, and current conversation are taught per site. The Chat view uses the active session.</p>
    <div class="card"><p>In Chat, each send continues the mock session. Teaching captures how a real site switches threads.</p></div>
  `;
}

function viewModes() {
  return `
    <h1>Modes</h1>
    <p class="sub">Policies independent of provider and model. Pick one in Chat.</p>
    <div class="grid">
      ${state.modes
        .map(
          (m) => `<article class="card"><h3>${esc(m.name)}</h3><p class="meta">${esc(m.instructions)}</p></article>`
        )
        .join("")}
    </div>
  `;
}

function viewSettings() {
  const c = state.status?.config || {};
  return `
    <h1>Settings</h1>
    <div class="card">
      <p>Data folder: ${esc(c.dataDir || "")}</p>
      <p>API port: ${c.port}</p>
      <p>AISetu is a desktop gateway. It is not an AI model.</p>
    </div>
  `;
}

function viewLogs() {
  const text = (state.logs || []).map((e) => `${e.ts}  ${e.level}  ${e.event}`).join("\n");
  return `<h1>Activity</h1><p class="sub">In-app event log. Secrets are not stored.</p><div class="log">${esc(text) || "Quiet so far."}</div>`;
}

function bind() {
  document.querySelectorAll("[data-nav]").forEach((b) =>
    b.addEventListener("click", () => {
      state.view = b.dataset.nav;
      if (state.view === "logs") {
        api("/api/logs").then((d) => {
          state.logs = d.entries;
          render();
        });
        return;
      }
      render();
    })
  );
  document.getElementById("addp")?.addEventListener("click", async () => {
    const name = document.getElementById("pname").value.trim();
    const url = document.getElementById("purl").value.trim();
    if (!name || !url) return;
    await api("/api/providers", { method: "POST", body: { name, url } });
    await refresh();
    render();
  });
  document.querySelectorAll("[data-select]").forEach((node) =>
    node.addEventListener("click", (e) => {
      if (e.target.dataset.del || e.target.dataset.open) return;
      state.selected = state.providers.find((p) => p.id === node.dataset.select);
      render();
    })
  );
  document.querySelectorAll("[data-open]").forEach((b) =>
    b.addEventListener("click", async () => {
      state.selected = state.providers.find((p) => p.id === b.dataset.open);
      await api(`/api/providers/${state.selected.id}/browser`, { method: "POST", body: {} });
      state.view = "browser";
      await refresh();
      render();
    })
  );
  document.querySelectorAll("[data-del]").forEach((b) =>
    b.addEventListener("click", async (e) => {
      e.stopPropagation();
      await api(`/api/providers/${b.dataset.del}`, { method: "DELETE" });
      if (state.selected?.id === b.dataset.del) state.selected = null;
      await refresh();
      render();
    })
  );
  document.getElementById("go")?.addEventListener("click", () => {
    document.getElementById("frame").src = document.getElementById("burl").value;
  });
  document.getElementById("authyes")?.addEventListener("click", async () => {
    await api(`/api/providers/${state.selected.id}/auth`, { method: "POST", body: { authenticated: true } });
    await refresh();
    render();
  });
  document.getElementById("authno")?.addEventListener("click", async () => {
    await api(`/api/providers/${state.selected.id}/auth`, { method: "POST", body: { authenticated: false } });
    await refresh();
    render();
  });
  document.getElementById("tstart")?.addEventListener("click", async () => {
    await api(`/api/providers/${state.selected.id}/teach/start`, { method: "POST", body: {} });
    await refresh();
    render();
  });
  document.getElementById("tconfirm")?.addEventListener("click", async () => {
    const notes = document.getElementById("tnotes")?.value;
    const raw = document.getElementById("modelnames")?.value || "";
    const models = raw
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean)
      .map((name) => ({ id: name.toLowerCase().replace(/[^a-z0-9]+/g, "-"), name }));
    await api(`/api/providers/${state.selected.id}/teach/confirm`, { method: "POST", body: { notes, models } });
    await refresh();
    render();
  });
  document.getElementById("tskip")?.addEventListener("click", async () => {
    await api(`/api/providers/${state.selected.id}/teach/skip`, { method: "POST", body: {} });
    await refresh();
    render();
  });
  document.getElementById("tval")?.addEventListener("click", async () => {
    await api(`/api/providers/${state.selected.id}/validate`, { method: "POST", body: {} });
    await refresh();
    render();
  });
  document.getElementById("csend")?.addEventListener("click", sendChat);
  document.getElementById("cinput")?.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendChat();
    }
  });
}

async function sendChat() {
  const input = document.getElementById("cinput");
  const text = input?.value.trim();
  if (!text || state.chat.streaming) return;
  state.chat.model = document.getElementById("cmodel").value;
  state.chat.mode = document.getElementById("cmode").value;
  input.value = "";
  state.chat.messages.push({ role: "user", content: text });
  const assistant = { role: "assistant", content: "" };
  state.chat.messages.push(assistant);
  state.chat.streaming = true;
  render();
  const res = await fetch("/v1/chat/completions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      model: state.chat.model,
      mode: state.chat.mode,
      stream: true,
      messages: state.chat.messages.filter((m) => m.content).map((m) => ({ role: m.role, content: m.content })),
    }),
  });
  const reader = res.body.getReader();
  const dec = new TextDecoder();
  let buf = "";
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    const parts = buf.split("\n\n");
    buf = parts.pop() || "";
    for (const part of parts) {
      const line = part.split("\n").find((l) => l.startsWith("data: "));
      if (!line) continue;
      const data = line.slice(6);
      if (data === "[DONE]") continue;
      try {
        const json = JSON.parse(data);
        const d = json.choices?.[0]?.delta?.content;
        if (d) assistant.content += d;
      } catch {
        /* ignore */
      }
    }
    const t = document.getElementById("transcript");
    if (t) {
      t.innerHTML = state.chat.messages.map((m) => `<div class="bubble ${m.role}">${esc(m.content)}</div>`).join("");
      t.scrollTop = t.scrollHeight;
    }
  }
  state.chat.streaming = false;
}

function esc(s) {
  return String(s ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/"/g, "&quot;");
}

refresh().then(render).catch((err) => {
  document.getElementById("app").textContent = err.message;
});
