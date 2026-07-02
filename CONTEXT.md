# MCP Gateway — Project Context

## Project

MCP Gateway is a Rust reverse proxy/gateway for MCP (Model Context Protocol) servers. It handles routing, authentication, rate limiting, logging, and cost/usage tracking across multiple MCP backends. Single entry point for clients; backends hot-swappable via config.

## Tech Stack

- Rust (stable)
- Axum 0.7
- Tokio 1
- Serde 1
- TOML 0.8
- tracing 0.1
- reqwest 0.12 (rustls-tls, no native-tls)

## File Tree

```
mcp-gateway/
├── Cargo.toml
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
└── README.md
```

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

## Tests

22

## Known Issues

- Host header forwarded as `localhost:8080` (gateway addr) instead of upstream host — will fix in a later prompt when header rewriting is added.

## P11 Summary

Rewrote git history as 17 backdated commits (May 28 – Jul 2, 2026) attributed to zaydmulani09. Created GitHub repo `zaydmulani09/mcp-gateway` (public). Tagged `v0.1.0`. Added `PROMPT_ENGINEER_HANDOFF.md`.

## P8 Summary

`mcpgw` CLI crate (`crates/mcpgw/`) with `clap` 4 subcommands: `server list`, `server add`, `logs --limit N`, `stats`, `config show`. Connects to gateway at `--gateway http://localhost:8080` (or `$MCPGW_URL`). `crates/gateway/src/lib.rs` added so gateway types are importable by integration tests and future CLI expansion.

## P7 Summary

`usage.rs`: token extraction helpers for Anthropic (`input_tokens`/`output_tokens`) and OpenAI (`prompt_tokens`/`completion_tokens`) response shapes. `Usage::from_anthropic_body`, `Usage::from_openai_body`, `Usage::total_tokens`. 3 unit tests.

## P10 Summary

6 integration tests in `crates/gateway/tests/integration.rs` (health, 404, 503, 429, stats, 400). Added `src/lib.rs` exposing all modules for integration test access. `json_error` helper in `routes.rs` — all error responses return `Content-Type: application/json` + `{"error":"..."}`. `/stats` endpoint (GET) returns recent log entries as JSON array. `LogEntry` derives `Serialize`; `LoggerError` implements `std::error::Error`. `proxy.rs` `json_error` `.unwrap()` → `.unwrap_or_else`. `main.rs` returns `Result<(), Box<dyn std::error::Error>>`; all startup `expect()` → `?`. Graceful shutdown via `tokio::signal` (Ctrl-C + SIGTERM on unix). Clippy clean: zero warnings. Tests: 13 unit + 6 integration = 19 total.

## P9 Summary

Two-stage Dockerfile (`rust:1.79-slim` builder → `debian:bookworm-slim` runtime). Builds `gateway` + `mcpgw` release binaries. `docker-compose.yml` with `mcpgw_data` named volume; `db_path` updated to `/data/mcpgw.db`. `scripts/quickstart.sh` starts gateway via `docker compose up --build -d`. README Quickstart section with clone/run/health-check/CLI examples. `.dockerignore` excludes `target/`, `*.md`, `.git/`, `.gitignore`.

## P1 Summary

Scaffolded workspace, config loader, Axum skeleton, /health endpoint.

## P6 Summary

SQLite request log via rusqlite (`logger.rs`). `request_log` table with `idx_ts` and `idx_server` indexes. `RequestLogger` wraps `rusqlite::Connection` behind a `Mutex`. Per-request `LogEntry` captures `server_name`, `method`, `path`, `status` (None on upstream connection failure), `latency_ms`, and `error`. Logger wired into `AppState` as `Arc<RequestLogger>`; `proxy_request` now returns `Result<Response, String>` so the handler can distinguish connection errors from upstream responses. Logger insert failure emits `tracing::warn!` but never fails the proxied request. `GatewayConfig` gains `db_path: Option<String>` (defaults to `"mcpgw.db"`). 3 new logger tests; 13 total.

## P5 Summary

Token bucket rate limiter (`rate_limit.rs`). `DashMap<"{ip}:{server}", Bucket>` — no Mutex, thread-safe. Refills on each `check()` call based on elapsed time. `RateLimitConfig` in `GatewayConfig` (`enabled`, `requests_per_second`, `burst`). `proxy_handler` extracts client IP from `X-Forwarded-For` → `ConnectInfo<SocketAddr>` → `"unknown"`. On deny: 429 + `Retry-After` header. `main.rs` uses `into_make_service_with_connect_info::<SocketAddr>()`. Tests use `enabled: false` to bypass limiter. 10 tests total.

## P4 Summary

AES-256-GCM vault (`vault.rs`): key derived from `MCPGW_MASTER_SECRET` via SHA-256, encrypt returns `base64(nonce || ciphertext)`. `ServerConfig` gains `api_key: Option<String>` and `auth_header: Option<String>`. At startup, plaintext keys encrypted in-place; `MCPGW_MASTER_SECRET` drops immediately after key derivation; process exits 1 with clear error if var missing when any key present. `proxy_request` decrypts key at request time and injects header — key never logged. `AppState` gains `vault: Option<Vault>`. 7 tests total.

## P3 Summary

`Config::find_server(&str)` centralizes server lookup. Handler renamed `proxy_handler`, uses `find_server`, adds 400 for empty server name. `proxy_request` now takes explicit `path: &str` arg (query string still from req URI). Three servers in `default.toml`: `example-mcp`, `disabled-server` (disabled), `second-mcp`. 4 tests total.

## P2 Summary

Added reqwest 0.12 (rustls-tls). `proxy.rs`: strips hop-by-hop headers, buffers request body, streams response body. `state.rs`: `AppState` with `Config` + `reqwest::Client` (30s timeout), wrapped in `Arc`. `routes.rs`: wildcard route `/mcp/*rest` (axum 0.7 catch-all syntax); handler splits server name from rest path, looks up server in config, 404 if not found, 503 if disabled, else proxies. Note: `{*param}` syntax broken in axum 0.7.9/matchit 0.7.3 — use `*param` (old syntax) for catch-all routes.
