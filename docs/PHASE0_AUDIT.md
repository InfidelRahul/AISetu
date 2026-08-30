# AISetu — Phase 0 Repository Audit

**Date:** 2026-08-30  
**Branch:** `arena/01a05365-aisetu`  
**HEAD:** `def7193049d8f3ec5b1bd62ec7357d483f6e4a6f` (`Initial commit`)  
**Scope:** Inspect repository and source. **No application features implemented in this phase.**

---

## 1. Current architecture

There is **no application architecture yet**.

| Item | Finding |
|------|---------|
| Source tree | Empty of application code |
| Files | `README.md` (`# AISetu`) only |
| Flutter app | Absent |
| Rust crate | Absent |
| FFI / plugin bridge | Absent |
| Tests | Absent |
| CI | Absent |
| Build system | Absent (no `pubspec.yaml`, `Cargo.toml`, CMake, Gradle, or Xcode project) |
| Persistence | Absent |
| Local API | Absent |
| Browser runtime | Absent |

The product exists only as a name in `README.md` and as the architecture described in the development master prompt. AISetu is a **greenfield** Flutter + Rust desktop bridge: OpenAI-compatible local API → provider runtime → embedded browser → user-visible web chat.

---

## 2. Technology stack

**Intended (from blueprint):**

| Layer | Technology | Role |
|-------|------------|------|
| UI | Flutter (desktop) | Providers, browser screen, teaching, models, conversations, modes, settings, logs |
| Core runtime | Rust | HTTP API, request pipeline, provider/conversation/model engines, persistence, logging, errors |
| Bridge | To be chosen in Phase 1 | Flutter ↔ Rust |
| Browser | To be chosen (see §4) | Embedded session per provider |

**Present in repo:** none of the above.

**Platform targets:** not declared. Desktop is implied (Windows / macOS / Linux). No mobile requirement.

---

## 3. Existing reusable components

**None in-repo.**

There is nothing to reuse except the product name in `README.md`. Phase 1 must introduce modules rather than wrap existing ones.

External ecosystems that *will* be reusable once added (not present now):

- Flutter `flutter_rust_bridge` or a thin FFI plugin
- Rust `axum` / `hyper` / `tiny_http` for local HTTP
- `serde` + JSON for profiles
- Structured logging (`tracing`)
- Desktop webview (see §4)

Do **not** pull those in during this audit.

---

## 4. Browser integration options

The runtime must be **provider-agnostic**, observe **user-visible text/controls**, inject text, click send, and detect generation state. Implementation details stay behind a browser abstraction.

| Option | Pros | Cons | Fit |
|--------|------|------|-----|
| **A. `webview_flutter` / `flutter_inappwebview` (Flutter-owned webview)** | UI embedding is natural; cookies/login stay in-process | Observation/automation APIs are thin; hard to snapshot semantic UI from Dart alone; mixing engine logic into Flutter violates the Rust-core boundary | Weak for the provider engine |
| **B. Native webview driven from Rust** (`wry`/`tao`, WebView2, WKWebView, WebKitGTK) | Session isolation per provider; JS eval for visible DOM text; closer to core runtime | Cross-platform packaging; embedding *inside* Flutter UI is awkward (two windows or texture glue) | Strong for runtime, weak for in-app browser *screen* |
| **C. Embedded Chromium via CDP** (e.g. `chromiumoxide` + system/bundled Chrome) | Excellent observation (a11y, DOM, network), streaming diffs, cancellation | Heavier; not “lightweight”; extra binary | High capability, conflicts with lightweight principle |
| **D. Hybrid: Flutter webview for login/teaching UI + Rust observation via injected JS / JS channel** | Matches UX (login in-app) and keeps engine in Rust | Fragile JS contract; must still normalize to a semantic page model | **Best Phase 4 target** if kept behind `BrowserRuntime` |
| **E. Headless Playwright/Puppeteer** | Mature automation | Separate process, not a desktop “browser screen”, heavy | Poor product fit |

**Recommendation (Phase 4, not Phase 1):** define a **provider-independent `BrowserRuntime` trait** in Rust (lifecycle, navigate, observe visible page, interact, events). Do not pick Chromium vs system webview until Phase 4. Phase 1 only needs the **module boundary and error types**, not a real browser.

**Risk:** Flutter’s in-widget webview vs Rust-owned window. Teaching Mode needs a visible browser *in the app*. That tension is the largest product/architecture risk (see §8).

---

## 5. Flutter / Rust integration options

| Option | Pros | Cons |
|--------|------|------|
| **`flutter_rust_bridge` (codegen)** | Idiomatic, typed, async, streams | Codegen in empty repo; learning curve |
| **`rinf`** | Flutter-first actor/signals | Less common |
| **Manual FFI (`dart:ffi` + `cdylib`)** | Minimal deps | Boilerplate, unsafe, poor streaming |
| **Local HTTP only (Flutter talks to Rust API on localhost)** | Matches product (API is first-class); simple | UI would share the public OpenAI surface or need extra admin routes; mixing admin + OpenAI on one port needs care |
| **Method channels via a thin native plugin** | Flutter-native | Extra Android/iOS/desktop plugin glue; desktop is the real target |

**Recommendation for Phase 1:**

- Rust `cdylib` + binary (API server can live in the same process).
- **`flutter_rust_bridge` v2** *or* a **small explicit FFI** plus **in-process API server** started from Rust.
- Keep a **clean boundary**: Flutter = screens and user gestures; Rust = domain, persistence, API, later browser.

**Do not** put provider engine, profiles, or OpenAI protocol in Dart.

Alternative that stays valid: Flutter starts a Rust sidecar process and uses HTTP for *all* control (admin JSON API + `/v1/*`). That simplifies embedding but complicates packaging and logs. Prefer **in-process** unless FRB proves too heavy.

---

## 6. Persistence options

Nothing exists. Profiles must be versioned, portable, inspectable JSON (blueprint §10).

| Option | Fit |
|--------|-----|
| **JSON files under app support dir** (`providers/<id>/profile.json`, `config.json`) | Best: portable, migratable, inspectable, no DB |
| SQLite | Useful later for logs; not needed for profiles |
| SharedPreferences / `flutter_secure_storage` | Wrong layer; secrets maybe later; not profiles |
| Platform keychain | Auth cookies/session — Phase 4/10, not Phase 1 |

**Phase 1:** app config directory + JSON config + log directory + a `Persistence` façade. **Do not** implement full `ProviderProfile` (Phase 3/9).

Cookie/session storage is a later session-management concern (Phase 4/26).

---

## 7. Local API implementation options

Blueprint Phase 2: bind **localhost**, `GET /v1/models`, `POST /v1/chat/completions`, validation, SSE streaming, cancellation, mock provider.

| Rust HTTP stack | Notes |
|-----------------|--------|
| **`axum` + `tokio` + `tower`** | Standard, SSE, cancellation via `Drop`/tokens; slightly heavier |
| **`hyper` only** | More wiring |
| **`tiny_http` / `warp`** | Smaller; weaker streaming/cancel story |

**Recommendation:** `axum` on `127.0.0.1` (configurable port) in **Phase 2**. Phase 1: **no server**, only module stubs / `ApiError` types if needed for the error model.

Flutter should not implement the OpenAI protocol.

---

## 8. Major technical risks

1. **Empty repo vs large blueprint** — easy to over-build Phase 1. Discipline: foundation only.
2. **Embedded browser vs lightweight** — real web AI UIs want a full browser; `wry` vs Chromium vs Flutter webview is unresolved until Phase 4.
3. **Flutter widget webview vs Rust engine** — Teaching UI needs a visible page; runtime needs scriptable observation. Hybrid JS channel may be fragile.
4. **Provider-agnostic observation** — semantic page model (visible text/controls) is underspecified; sites are highly dynamic (shadow DOM, canvases, virtual lists).
5. **Login/session isolation** — cookies, 2FA, captchas; sessions must not leak across providers.
6. **Streaming from DOM diffs** — rate, flicker, markdown re-render can break deltas.
7. **Cancellation** — must click “stop” in a foreign UI; may fail.
8. **Desktop packaging** — Flutter desktop + native webview + Rust lib on Win/macOS/Linux.
9. **Legal/ToS** — automating third-party chat UIs; product risk, not a code blocker for Phase 1.
10. **Naming** — keep `AISetu` out of type names (blueprint §4).

---

## 9. Gaps against the blueprint

Every numbered capability is a gap. Mapping:

| Blueprint area | Status |
|----------------|--------|
| Flutter UI (providers, browser, teaching, …) | Missing |
| Rust core (API, engines, registry, composer, observer) | Missing |
| Flutter/Rust boundary | Missing |
| Domain: Provider, Model, Mode, Conversation | Missing |
| Teaching mode / profile / replay / validation | Missing |
| Browser runtime / visible UI model | Missing |
| Response observer / stream normalizer | Missing |
| Conversation engine / model registry / modes | Missing |
| Context builder / prompt composer | Missing |
| OpenAI-compatible API | Missing |
| Cancellation pipeline | Missing |
| Session management | Missing |
| Structured errors and logging | Missing |
| Tests | Missing |
| Phases 1–17 implementation | Not started |

**No architectural conflict in code** — there is no code. The only design tension to resolve later: **where the browser lives** (Flutter widget vs Rust window) without collapsing engines into the UI.

---

## 10. Recommended Phase 1 architecture

Phase 1 = **buildable foundation**, not API, not browser, not teaching.

```
AISetu/
  README.md
  docs/PHASE0_AUDIT.md
  app/                          # Flutter desktop application
    lib/
      main.dart
      app.dart                  # shell: nav placeholders
      bridge/                   # generated or thin FRB/FFI wrappers
      ui/                       # placeholder screens only
        home_shell.dart
        status_page.dart
    pubspec.yaml
    ... desktop platform folders
  native/                       # or crates/
    core/                       # Rust library (cdylib + rlib)
      src/
        lib.rs
        domain/                 # types only: ProviderId, ModeId, …
        error.rs                # structured errors (enum)
        logging.rs              # tracing setup
        config.rs               # app paths, port, log level
        persist.rs              # data-dir + JSON read/write helpers
        bridge.rs               # FFI-facing init/shutdown/status
      Cargo.toml
      tests/
  rust-toolchain.toml           # optional pin
```

**Phase 1 responsibilities**

1. Flutter desktop project (Linux/Windows/macOS as the sandbox allows).
2. Rust `core` crate linked into Flutter.
3. Init path: Flutter start → `core::init()` → logging + config dir.
4. Domain **identifiers and enums only** (no repositories, no teaching).
5. Structured `Error` enum covering names from blueprint §27 (variants may be unused).
6. Structured logging (`tracing`) without secrets.
7. Persistence **foundation**: resolve data directory, read/write JSON blob helper — not provider profiles.
8. Tests: error Display/serde, config path, persist round-trip.
9. Placeholder UI: app name, status “core initialized”, empty nav for future areas — **not** teaching/browser.

**Explicitly out of Phase 1**

- HTTP server, `/v1/*`
- Browser, webview
- Teaching, profiles, validation
- Model registry contents, conversation engine, modes beyond enum stubs
- Context/prompt pipeline
- Mock provider (Phase 2)

**Boundary rule:** Dart does not own domain persistence or errors; it displays what Rust returns.

---

## 11. Concrete files / modules for Phase 1

**Create**

| Path | Purpose |
|------|---------|
| `app/pubspec.yaml` | Flutter desktop app `aisetu` (product name in metadata/UI only) |
| `app/lib/main.dart` | Entry; call Rust init |
| `app/lib/app.dart` | Material/Cupertino shell, placeholder destinations |
| `app/lib/bridge/` | FFI/FRB bindings |
| `native/core/Cargo.toml` | `core` crate: serde, thiserror, tracing, tokio (if needed for init) |
| `native/core/src/lib.rs` | `init`, `shutdown`, `status` |
| `native/core/src/error.rs` | `Error` enum (`ProviderNotConfigured`, `AuthenticationRequired`, …) |
| `native/core/src/logging.rs` | `tracing` subscriber, file + stderr |
| `native/core/src/config.rs` | `AppConfig`, data/log dirs, defaults |
| `native/core/src/persist.rs` | atomic JSON write/read |
| `native/core/src/domain/mod.rs` | `ProviderId`, `ModelId`, `Mode` (Ask/Code/Debug/Plan/Review), `ConversationId` newtypes |
| `native/core/tests/*.rs` | persist + error + config tests |
| `docs/` | keep this audit |

**Modify**

| Path | Change |
|------|--------|
| `README.md` | Short product description + how to run Flutter/Rust **after** Phase 1 exists |

**Do not create in Phase 1**

- `api/`, `axum` server, OpenAI types
- `browser/`, webview plugins
- `teaching/`, `profile/`, `observer/`
- Provider repository implementations beyond empty traits (prefer **no traits until Phase 3**)

---

## Audit conclusion

AISetu is a **greenfield** repository. Phase 0 finds **no reusable application code**, **no stack**, and **no conflicts**—only an empty tree versus a large target architecture.

**Phase 1 should establish Flutter + Rust, errors, logging, config, persistence helpers, and domain IDs—then stop.**

This audit is complete. **Phase 1 is not started.**
