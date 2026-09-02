# AISetu

AISetu is a provider-independent AI conversation bridge. It translates canonical
conversations through provider adapters, and exposes an OpenAI-compatible local API.

```
OpenAI Request
      ↓
API
      ↓
Canonical Conversation
      ↓
Provider Adapter
      ↓
HTTP
      ↓
Provider
      ↓
Conversation
      ↓
OpenAI Response
```

## Build

Requires a stable Rust toolchain (1.75+).

```bash
cargo build
cargo test
cargo fmt
cargo clippy
```

Release profile (thin LTO, stripped):

```bash
cargo build --release -p aisetu
```

Cross-platform artifacts (Linux/Windows/macOS, x86_64 and ARM64) are produced
by `.github/workflows/ci.yml` and `scripts/build-release.sh`.

## Run

```bash
# Start the local API (127.0.0.1:8080 by default)
cargo run -p aisetu -- serve

# Inspect resolved configuration
cargo run -p aisetu -- config

# Capture a provider session via the browser plugin
cargo run -p aisetu -- login --provider mock --url https://example.com/login --cookie sid=abc
```

Optional configuration file: `config/aisetu.toml`, `./aisetu.toml`, or
`AISETU_CONFIG`. Environment overrides: `AISETU_BIND`, `AISETU_PORT`,
`AISETU_API_KEY`, `AISETU_LOG_LEVEL`, `AISETU_LOG_FORMAT`.

## API

- `GET /health`
- `GET /v1/models`
- `POST /v1/chat/completions` (including `stream: true`)

Default models:

- `aisetu-default` → mock provider
- `aisetu-echo` → echo provider

```bash
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"aisetu-default","messages":[{"role":"user","content":"hello"}]}'
```

## Workspace

| Crate | Responsibility |
| --- | --- |
| `aisetu-core` | Errors, config, tracing, limits, redaction |
| `aisetu-transport` | Generic HTTP transport |
| `aisetu-conversation` | Canonical conversation model |
| `aisetu-engine` | Translation / extract / normalize / validate |
| `aisetu-intelligence` | IntelligenceEngine + Needle |
| `aisetu-provider` | Adapters, capabilities, routing, reliability |
| `aisetu-session` | Session lifecycle and secret storage |
| `aisetu-browser` | Replaceable authentication bridge |
| `aisetu-api` | HTTP API + OpenAI endpoints |
| `aisetu` | Binary, wiring, release entrypoint |

## License

MIT
