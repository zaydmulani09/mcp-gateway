# MCP Gateway — Prompt Engineer Handoff

## What Is MCP Gateway

Self-hosted reverse proxy / gateway for MCP (Model Context Protocol) servers.
Single entry point for AI clients; routes requests to named MCP backends with:
- Per-server API key injection (AES-256-GCM encrypted at rest)
- Token bucket rate limiting per client IP + server
- SQLite request logging with latency and error capture
- Token usage extraction (Anthropic and OpenAI response formats)
- `mcpgw` CLI for log inspection and server management
- Docker-first deployment (`docker compose up --build`)

## Tech Stack

| Component | Version |
|-----------|---------|
| Rust | stable (1.79+) |
| axum | 0.7 |
| tokio | 1 (full) |
| serde | 1 |
| toml | 0.8 |
| reqwest | 0.12 (rustls-tls, no native-tls) |
| aes-gcm | 0.10 |
| dashmap | 6 |
| rusqlite | 0.31 (bundled SQLite) |
| chrono | 0.4 |
| uuid | 1 |
| clap | 4 (mcpgw CLI) |

## Prompt Status

| Prompt | Title | Status |
|--------|-------|--------|
| P1 | Project scaffold | ✅ |
| P2 | MCP connection layer | ✅ |
| P3 | Multi-server routing | ✅ |
| P4 | Auth layer | ✅ |
| P5 | Rate limiting | ✅ |
| P6 | Request logging | ✅ |
| P7 | Cost/usage tracking | ✅ |
| P8 | CLI tool | ✅ |
| P9 | Docker packaging | ✅ |
| P10 | Tests + polish | ✅ |
| P11 | GitHub setup | ✅ |

## File Tree

```
mcp-gateway/
├── Cargo.toml
├── Cargo.lock
├── .gitignore
├── Dockerfile
├── .dockerignore
├── docker-compose.yml
├── scripts/
│   └── quickstart.sh
├── crates/
│   ├── gateway/
│   │   ├── Cargo.toml
│   │   ├── tests/
│   │   │   └── integration.rs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       ├── config.rs
│   │       ├── health.rs
│   │       ├── proxy.rs
│   │       ├── rate_limit.rs
│   │       ├── logger.rs
│   │       ├── usage.rs
│   │       ├── routes.rs
│   │       ├── state.rs
│   │       └── vault.rs
│   ├── mcpgw/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   └── common/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── config/
│   └── default.toml
├── CONTEXT.md
├── PROMPT_ENGINEER_HANDOFF.md
└── README.md
```

## Tests

22 total: 19 existing (13 unit + 6 integration) + 3 usage unit tests

Run: `cargo +stable-x86_64-pc-windows-msvc test --all`

## Known Issues / Tech Debt

- **Host header** forwarded as `localhost:8080` (gateway addr) instead of upstream host — header rewriting not yet implemented
- **usage.rs** parses token counts from response bodies but the counts are not persisted to SQLite yet
- **mcpgw CLI** `server list` reads from filesystem config rather than calling a live gateway API endpoint; `server add` prints TOML snippet rather than writing the file
- **No TLS termination** at the gateway level — relies on upstream TLS via reqwest/rustls
- **SQLite path mismatch** — defaults to `mcpgw.db` locally but Docker uses `/data/mcpgw.db`; no migration tooling

## P12 Ideas

- **Web dashboard** — serve a small HTML page at `/dashboard` showing `/stats` in a table, auto-refreshing every 30s
- **Webhook alerts** — POST to a configured URL when rate-limit events or upstream errors exceed thresholds
- **Multi-tenant API keys** — issue per-client bearer tokens at the gateway level, mapping to upstream servers
- **Token cost tracking** — persist `usage.input_tokens` / `usage.output_tokens` per request to SQLite; expose `/costs?server=X&from=Y&to=Z`
- **Config reload** — SIGHUP triggers hot-reload of `config/default.toml` without restart
- **Metrics** — expose Prometheus-compatible `/metrics` endpoint

## Starting a New Session

Paste both `CONTEXT.md` and `PROMPT_ENGINEER_HANDOFF.md` into the context window, then describe the next feature.

The project was built entirely via structured prompts (P1–P11). Future prompts should follow the same pattern: one numbered prompt = one focused feature or polish pass.
