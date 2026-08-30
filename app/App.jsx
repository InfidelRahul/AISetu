import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
  useWindowDimensions,
} from "react-native";
import { api, streamChat } from "./api.js";
import { colors } from "./theme.js";
import { getBase, setBase, platformLabel, defaultBase } from "./platform.js";

const NAV = [
  ["providers", "Providers"],
  ["browser", "Browser"],
  ["teaching", "Teaching"],
  ["chat", "Chat"],
  ["models", "Models"],
  ["conversations", "Conversations"],
  ["modes", "Modes"],
  ["settings", "Settings"],
  ["logs", "Activity"],
];

export default function App() {
  const { width } = useWindowDimensions();
  const compact = width < 880;
  const [view, setView] = useState("providers");
  const [providers, setProviders] = useState([]);
  const [selectedId, setSelectedId] = useState(null);
  const [modes, setModes] = useState([]);
  const [status, setStatus] = useState(null);
  const [meta, setMeta] = useState({ copy: {}, stages: [] });
  const [logs, setLogs] = useState([]);
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [notes, setNotes] = useState("");
  const [modelNames, setModelNames] = useState("");
  const [browserUrl, setBrowserUrl] = useState("");
  const [chatModel, setChatModel] = useState("mock/mock-fast");
  const [chatMode, setChatMode] = useState("ask");
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState([]);
  const [error, setError] = useState("");
  const [apiHost, setApiHost] = useState(() => getBase() || defaultBase() || "http://127.0.0.1:8787");

  const selected = providers.find((p) => p.id === selectedId) || null;

  const refresh = useCallback(async () => {
    const [plist, m, st, te] = await Promise.all([
      api("/api/providers"),
      api("/api/modes"),
      api("/api/status"),
      api("/api/teaching/meta"),
    ]);
    setProviders(plist.providers);
    setModes(m.modes);
    setStatus(st);
    setMeta(te);
  }, []);

  useEffect(() => {
    refresh().catch((e) => setError(e.message));
  }, [refresh]);

  useEffect(() => {
    if (selected) setBrowserUrl(selected.url);
  }, [selectedId]);

  const modelOptions = useMemo(() => {
    const list = ["mock/mock-fast", "mock/mock-reason"];
    for (const p of providers) {
      for (const m of p.profile?.modelStrategy?.models || [{ id: "default" }]) {
        list.push(`${p.id}/${m.id}`);
      }
    }
    return list;
  }, [providers]);

  async function addProvider() {
    if (!name.trim() || !url.trim()) return;
    const rec = await api("/api/providers", { method: "POST", body: { name: name.trim(), url: url.trim() } });
    setName("");
    setUrl("");
    await refresh();
    setSelectedId(rec.id);
  }

  async function openProvider(id) {
    await api(`/api/providers/${id}/browser`, { method: "POST", body: {} });
    setSelectedId(id);
    setView("browser");
    await refresh();
  }

  async function removeProvider(id) {
    await api(`/api/providers/${id}`, { method: "DELETE" });
    if (selectedId === id) setSelectedId(null);
    await refresh();
  }

  async function setAuth(yes) {
    if (!selected) return;
    await api(`/api/providers/${selected.id}/auth`, { method: "POST", body: { authenticated: yes } });
    await refresh();
  }

  async function teach(action) {
    if (!selected) return;
    const path =
      action === "start"
        ? "teach/start"
        : action === "skip"
          ? "teach/skip"
          : action === "validate"
            ? "validate"
            : "teach/confirm";
    const models = modelNames
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean)
      .map((n) => ({ id: n.toLowerCase().replace(/[^a-z0-9]+/g, "-"), name: n }));
    await api(`/api/providers/${selected.id}/${path}`, {
      method: "POST",
      body: { notes, models },
    });
    await refresh();
  }

  async function send() {
    const text = draft.trim();
    if (!text) return;
    setDraft("");
    const next = [...messages, { role: "user", content: text }, { role: "assistant", content: "" }];
    setMessages(next);
    await streamChat({
      model: chatModel,
      mode: chatMode,
      messages: next.filter((m) => m.content).map((m) => ({ role: m.role, content: m.content })),
      onDelta: (d) => {
        setMessages((cur) => {
          const copy = cur.slice();
          const last = { ...copy[copy.length - 1] };
          last.content += d;
          copy[copy.length - 1] = last;
          return copy;
        });
      },
    });
  }

  async function openLogs() {
    const d = await api("/api/logs");
    setLogs(d.entries || []);
    setView("logs");
  }

  const step = selected?.teaching?.step;
  const copy = meta.copy?.[step] || {
    title: "Start teaching",
    body: "Walk through how this site’s visible chat works.",
  };
  const report = selected?.validation?.report || {};

  return (
    <View style={styles.shell}>
      <View style={styles.titlebar}>
        <View style={styles.dots}>
          <View style={[styles.dot, { backgroundColor: "#c45c4a" }]} />
          <View style={[styles.dot, { backgroundColor: "#c9a227" }]} />
          <View style={[styles.dot, { backgroundColor: "#3d8f5c" }]} />
        </View>
        <Text style={styles.titleName}>AISetu</Text>
        <Text style={styles.titleMeta}>{platformLabel}</Text>
      </View>
      <View style={[styles.workspace, compact && styles.workspaceCompact]}>
        <View style={[styles.nav, compact && styles.navCompact]}>
          <Text style={styles.brand}>AISetu</Text>
          <Text style={styles.tag}>Web chat, as an API</Text>
          {NAV.map(([id, label]) => (
            <Pressable
              key={id}
              onPress={() => (id === "logs" ? openLogs() : setView(id))}
              style={[styles.navBtn, view === id && styles.navBtnActive]}
            >
              <Text style={styles.navTxt}>{label}</Text>
            </Pressable>
          ))}
          <Text style={styles.chip}>
            Local API on port {status?.config?.port || 8787}. This window is the app — not a terminal.
          </Text>
        </View>
        <ScrollView style={styles.page} contentContainerStyle={styles.pageInner}>
          {error ? <Text style={styles.err}>{error}</Text> : null}

          {view === "providers" && (
            <>
              <Text style={styles.h1}>Providers</Text>
              <Text style={styles.sub}>Add a website, sign in, teach the UI, then chat here or from an IDE.</Text>
              <View style={styles.card}>
                <Field label="Name" value={name} onChangeText={setName} placeholder="Work chat" />
                <Field label="Website URL" value={url} onChangeText={setUrl} placeholder="https://chat.example.com" />
                <Btn title="Add provider" primary onPress={addProvider} />
              </View>
              <View style={styles.grid}>
                {providers.map((p) => (
                  <Pressable key={p.id} onPress={() => setSelectedId(p.id)} style={styles.card}>
                    <Text style={styles.h3}>{p.name}</Text>
                    <Text style={styles.meta}>{p.url}</Text>
                    <Text style={styles.meta}>
                      {p.status || "draft"} {p.authenticated ? "· signed in" : ""}
                    </Text>
                    <View style={styles.row}>
                      <Btn title="Open" primary onPress={() => openProvider(p.id)} />
                      <Btn title="Remove" onPress={() => removeProvider(p.id)} />
                    </View>
                  </Pressable>
                ))}
              </View>
            </>
          )}

          {view === "browser" && (
            <>
              <Text style={styles.h1}>{selected ? selected.name : "Browser"}</Text>
              {!selected ? (
                <Text style={styles.sub}>Select a provider first.</Text>
              ) : (
                <>
                  <Text style={styles.sub}>Sign in inside this pane. Mark signed-in when the chat UI is ready.</Text>
                  <View style={styles.row}>
                    <TextInput style={[styles.input, { flex: 1 }]} value={browserUrl} onChangeText={setBrowserUrl} />
                    <Btn title="Signed in" primary onPress={() => setAuth(true)} />
                    <Btn title="Signed out" onPress={() => setAuth(false)} />
                  </View>
                  <BrowserFrame url={browserUrl || selected.url} />
                </>
              )}
            </>
          )}

          {view === "teaching" && (
            <>
              <Text style={styles.h1}>Teaching</Text>
              {!selected ? (
                <Text style={styles.sub}>Select a provider first.</Text>
              ) : (
                <View style={styles.split}>
                  <View style={styles.card}>
                    <Text style={styles.h3}>{copy.title}</Text>
                    <Text style={styles.sub}>{copy.body}</Text>
                    {step === "chatInput" ? (
                      <Text style={styles.meta}>Test phrase: {selected.teaching?.testMessage}</Text>
                    ) : null}
                    {step === "models" ? (
                      <Field label="Visible models" value={modelNames} onChangeText={setModelNames} placeholder="Qwen3-Coder, Fast" />
                    ) : null}
                    <Field label="Notes" value={notes} onChangeText={setNotes} />
                    <View style={styles.row}>
                      <Btn title="Start" primary onPress={() => teach("start")} />
                      <Btn title="Confirm" primary onPress={() => teach("confirm")} />
                      <Btn title="Skip" onPress={() => teach("skip")} />
                      <Btn title="Validate" onPress={() => teach("validate")} />
                    </View>
                  </View>
                  <View style={styles.card}>
                    <Text style={styles.h3}>Capabilities</Text>
                    {Object.entries(report).map(([k, v]) => (
                      <Text key={k} style={styles.meta}>
                        {k}  {v ? "✓" : "—"}
                      </Text>
                    ))}
                    <Text style={styles.meta}>{selected.validation?.passed ? "ready" : "not ready"}</Text>
                  </View>
                </View>
              )}
            </>
          )}

          {view === "chat" && (
            <>
              <Text style={styles.h1}>Chat</Text>
              <Text style={styles.sub}>Streaming inside the desktop app. Mock models work immediately.</Text>
              <View style={styles.row}>
                <Field label="Model" value={chatModel} onChangeText={setChatModel} />
                <Field label="Mode" value={chatMode} onChangeText={setChatMode} />
              </View>
              <View style={styles.chat}>
                <ScrollView style={styles.transcript}>
                  {messages.map((m, i) => (
                    <View key={i} style={[styles.bubble, m.role === "user" ? styles.user : styles.bot]}>
                      <Text style={m.role === "user" ? styles.userTxt : styles.botTxt}>{m.content}</Text>
                    </View>
                  ))}
                </ScrollView>
                <View style={styles.composer}>
                  <TextInput
                    style={[styles.input, { flex: 1 }]}
                    value={draft}
                    onChangeText={setDraft}
                    placeholder="Message"
                    onSubmitEditing={send}
                  />
                  <Btn title="Send" primary onPress={send} />
                </View>
              </View>
              <Text style={styles.meta}>Models: {modelOptions.join(" · ")}</Text>
            </>
          )}

          {view === "models" && (
            <>
              <Text style={styles.h1}>Models</Text>
              <Text style={styles.sub}>Discovered from teaching, not a hard-coded vendor list.</Text>
              <View style={styles.card}>
                {(selected?.profile?.modelStrategy?.models || []).map((m) => (
                  <Text key={m.id} style={styles.meta}>
                    {selected.id}/{m.id} — {m.name}
                  </Text>
                ))}
                {!selected ? <Text style={styles.meta}>Use mock/mock-fast in Chat, or select a provider.</Text> : null}
              </View>
            </>
          )}

          {view === "conversations" && (
            <>
              <Text style={styles.h1}>Conversations</Text>
              <Text style={styles.sub}>Teaching captures new chat, history, open, and current conversation per site.</Text>
            </>
          )}

          {view === "modes" && (
            <>
              <Text style={styles.h1}>Modes</Text>
              <Text style={styles.sub}>Policies, independent of provider and model.</Text>
              <View style={styles.grid}>
                {modes.map((m) => (
                  <View key={m.id} style={styles.card}>
                    <Text style={styles.h3}>{m.name}</Text>
                    <Text style={styles.meta}>{m.instructions}</Text>
                  </View>
                ))}
              </View>
            </>
          )}

          {view === "settings" && (
            <>
              <Text style={styles.h1}>Settings</Text>
              <View style={styles.card}>
                <Text style={styles.meta}>This device: {platformLabel}</Text>
                <Text style={styles.meta}>Data: {status?.config?.dataDir || "via API host"}</Text>
                <Text style={styles.meta}>Port: {status?.config?.port || "—"}</Text>
                <Field
                  label="API host (Android uses 10.0.2.2 for the emulator, or your PC LAN IP)"
                  value={apiHost}
                  onChangeText={setApiHost}
                />
                <Btn
                  title="Save API host"
                  primary
                  onPress={() => {
                    setBase(apiHost);
                    refresh().catch((e) => setError(e.message));
                  }}
                />
                <Text style={styles.sub}>
                  Windows, macOS, and Linux run the full gateway in Electron. Android/iOS are the same React Native
                  app and talk to that gateway over the network.
                </Text>
              </View>
            </>
          )}

          {view === "logs" && (
            <>
              <Text style={styles.h1}>Activity</Text>
              <View style={styles.logBox}>
                {(logs || []).map((e, i) => (
                  <Text key={i} style={styles.logLine}>
                    {e.ts}  {e.level}  {e.event}
                  </Text>
                ))}
              </View>
            </>
          )}
        </ScrollView>
      </View>
    </View>
  );
}

function Field({ label, ...rest }) {
  return (
    <View style={{ flex: 1, minWidth: 160, marginBottom: 8 }}>
      <Text style={styles.label}>{label}</Text>
      <TextInput style={styles.input} {...rest} />
    </View>
  );
}

function Btn({ title, onPress, primary }) {
  return (
    <Pressable onPress={onPress} style={[styles.btn, primary && styles.btnPrimary]}>
      <Text style={[styles.btnTxt, primary && styles.btnTxtPrimary]}>{title}</Text>
    </Pressable>
  );
}

function BrowserFrame({ url }) {
  return (
    <View style={styles.frame}>
      <iframe title="provider" src={url} style={{ border: 0, width: "100%", height: "56vh", background: "#fff" }} />
    </View>
  );
}

const styles = StyleSheet.create({
  shell: { flex: 1, backgroundColor: colors.nav, height: "100%" },
  titlebar: {
    height: 40,
    backgroundColor: colors.title,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    paddingHorizontal: 12,
  },
  dots: { flexDirection: "row", gap: 7 },
  dot: { width: 11, height: 11, borderRadius: 6 },
  titleName: { color: colors.cream, fontWeight: "600", letterSpacing: 0.6 },
  titleMeta: { color: "#cbbba6", fontSize: 12 },
  workspace: { flex: 1, flexDirection: "row", backgroundColor: colors.paper, minHeight: 0 },
  workspaceCompact: { flexDirection: "column" },
  nav: { width: 212, backgroundColor: colors.nav, padding: 14 },
  navCompact: { width: "100%", flexDirection: "row", flexWrap: "wrap" },
  brand: { color: colors.cream, fontSize: 22, fontWeight: "700" },
  tag: { color: "#b5a894", fontSize: 11, marginBottom: 16, marginTop: 2 },
  navBtn: { paddingVertical: 9, paddingHorizontal: 10, borderRadius: 8, marginBottom: 2 },
  navBtnActive: { backgroundColor: "#322c26" },
  navTxt: { color: colors.cream, fontSize: 14 },
  chip: { color: "#cbbba6", fontSize: 10, marginTop: 18, lineHeight: 14 },
  page: { flex: 1 },
  pageInner: { padding: 24, paddingBottom: 48 },
  h1: { fontSize: 28, fontWeight: "700", color: colors.ink, marginBottom: 6 },
  h3: { fontSize: 17, fontWeight: "700", color: colors.ink, marginBottom: 4 },
  sub: { color: colors.muted, marginBottom: 14, fontSize: 14, lineHeight: 20 },
  card: {
    backgroundColor: colors.panel,
    borderColor: colors.line,
    borderWidth: 1,
    borderRadius: 12,
    padding: 14,
    marginBottom: 12,
  },
  grid: { flexDirection: "row", flexWrap: "wrap", gap: 12 },
  row: { flexDirection: "row", flexWrap: "wrap", gap: 8, alignItems: "center", marginTop: 8 },
  split: { flexDirection: "row", flexWrap: "wrap", gap: 12 },
  label: { fontSize: 12, color: colors.muted, marginBottom: 4 },
  input: {
    backgroundColor: "#fff",
    borderColor: colors.line,
    borderWidth: 1,
    borderRadius: 8,
    paddingHorizontal: 10,
    paddingVertical: 8,
    fontSize: 14,
    color: colors.ink,
  },
  btn: {
    backgroundColor: "#fff",
    borderColor: colors.line,
    borderWidth: 1,
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 8,
    marginTop: 8,
  },
  btnPrimary: { backgroundColor: colors.accent, borderColor: "#9a3d18" },
  btnTxt: { color: colors.ink, fontSize: 13 },
  btnTxtPrimary: { color: "#fff" },
  meta: { color: colors.muted, fontSize: 12, marginTop: 4 },
  err: { color: "#8d2d2d", marginBottom: 8 },
  frame: { marginTop: 10, borderRadius: 12, overflow: "hidden", borderWidth: 1, borderColor: colors.line },
  chat: {
    minHeight: 420,
    borderWidth: 1,
    borderColor: colors.line,
    borderRadius: 12,
    backgroundColor: colors.panel,
    overflow: "hidden",
  },
  transcript: { height: 340, padding: 12 },
  bubble: { maxWidth: "78%", padding: 10, borderRadius: 12, marginBottom: 8 },
  user: { alignSelf: "flex-end", backgroundColor: colors.teal },
  bot: { alignSelf: "flex-start", backgroundColor: "#fff", borderWidth: 1, borderColor: colors.line },
  userTxt: { color: colors.paper },
  botTxt: { color: colors.ink },
  composer: { flexDirection: "row", gap: 8, padding: 10, borderTopWidth: 1, borderTopColor: colors.line },
  logBox: { backgroundColor: colors.nav, borderRadius: 8, padding: 12 },
  logLine: { color: colors.cream, fontFamily: "monospace", fontSize: 11, marginBottom: 4 },
});
