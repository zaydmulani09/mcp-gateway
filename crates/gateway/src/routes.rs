use std::{net::SocketAddr, sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use serde_json::json;

use crate::{
    health, proxy,
    rate_limit::RateLimitResult,
    state::AppState,
};

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/stats", get(stats_handler))
        .route("/mcp/*rest", any(proxy_handler))
        .with_state(state)
}

fn json_error(status: StatusCode, message: &str) -> Response {
    let body = serde_json::to_string(&json!({ "error": message }))
        .unwrap_or_else(|_| format!(r#"{{"error":"{}"}}"#, message));
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> Response {
    match state.logger.recent(100) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => {
            tracing::error!("stats query failed: {:?}", e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to query stats")
        }
    }
}

async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    addr: Option<ConnectInfo<SocketAddr>>,
    Path(rest): Path<String>,
    req: Request,
) -> Response {
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| addr.map(|ci| ci.0.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let (server_name, sub_path) = match rest.split_once('/') {
        Some((name, path)) => (name.to_string(), format!("/{}", path)),
        None => (rest.clone(), "/".to_string()),
    };

    if server_name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "missing server name");
    }

    match state.config.find_server(&server_name) {
        None => json_error(StatusCode::NOT_FOUND, "server not found"),
        Some(s) if !s.enabled => json_error(StatusCode::SERVICE_UNAVAILABLE, "server disabled"),
        Some(s) => match state.rate_limiter.check(&client_ip, &server_name) {
            RateLimitResult::Denied { retry_after_secs } => {
                let retry_ceil = retry_after_secs.ceil() as u64;
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(axum::http::header::RETRY_AFTER, retry_ceil.to_string())],
                    Json(json!({
                        "error": "rate limit exceeded",
                        "retry_after_secs": retry_after_secs
                    })),
                )
                    .into_response()
            }
            RateLimitResult::Allowed => {
                let start = Instant::now();
                let method = req.method().to_string();
                let auth_header = s.auth_header.as_deref().unwrap_or("Authorization");
                let auth = s.api_key.as_deref().map(|key| (auth_header, key));

                let proxy_result = proxy::proxy_request(
                    &state.http_client,
                    &s.url,
                    &sub_path,
                    auth,
                    state.vault.as_ref(),
                    req,
                )
                .await;

                let latency_ms = start.elapsed().as_millis() as i64;

                let (response, status, error) = match proxy_result {
                    Ok(resp) => {
                        let code = resp.status().as_u16() as i64;
                        (resp, Some(code), None)
                    }
                    Err(err_msg) => {
                        let resp = json_error(StatusCode::SERVICE_UNAVAILABLE, &err_msg);
                        (resp, None, Some(err_msg))
                    }
                };

                let entry = crate::logger::LogEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    ts: chrono::Utc::now().to_rfc3339(),
                    server_name,
                    client_ip,
                    method,
                    path: sub_path,
                    status,
                    latency_ms: Some(latency_ms),
                    error,
                };
                if let Err(e) = state.logger.insert(&entry) {
                    tracing::warn!("logger insert failed: {:?}", e);
                }

                response
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, GatewayConfig, RateLimitConfig, ServerConfig},
        logger::RequestLogger,
        rate_limit::RateLimiter,
        state::AppState,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn make_state(servers: Vec<ServerConfig>) -> Arc<AppState> {
        let rl_config = RateLimitConfig {
            enabled: false,
            requests_per_second: 10.0,
            burst: 20.0,
        };
        Arc::new(AppState {
            config: Config {
                gateway: GatewayConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8080,
                    log_level: "error".to_string(),
                    rate_limit: rl_config.clone(),
                    db_path: None,
                },
                servers,
            },
            http_client: reqwest::Client::new(),
            vault: None,
            rate_limiter: Arc::new(RateLimiter::new(&rl_config)),
            logger: Arc::new(RequestLogger::new(":memory:").unwrap()),
        })
    }

    #[tokio::test]
    async fn disabled_server_returns_503() {
        let state = make_state(vec![ServerConfig {
            name: "disabled-server".to_string(),
            url: "http://localhost:3001".to_string(),
            enabled: false,
            api_key: None,
            auth_header: None,
        }]);

        let app = build_router(state);
        let req = Request::builder()
            .uri("/mcp/disabled-server/tools/list")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn missing_server_name_returns_400() {
        let state = make_state(vec![]);

        let app = build_router(state);
        let req = Request::builder()
            .uri("/mcp//tools/list")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn find_server_returns_none_for_unknown() {
        let rl_config = RateLimitConfig {
            enabled: false,
            requests_per_second: 10.0,
            burst: 20.0,
        };
        let config = Config {
            gateway: GatewayConfig {
                host: "".to_string(),
                port: 8080,
                log_level: "".to_string(),
                rate_limit: rl_config,
                db_path: None,
            },
            servers: vec![],
        };
        assert!(config.find_server("nonexistent").is_none());
    }
}
