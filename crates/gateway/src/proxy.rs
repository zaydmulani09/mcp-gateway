use axum::{body::Body, http::StatusCode, response::Response};
use serde_json::json;

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "transfer-encoding",
    "te",
    "upgrade",
    "proxy-authorization",
    "proxy-authenticate",
];

/// Returns `Ok(response)` for any response (including internal error responses),
/// and `Err(msg)` only when the upstream TCP/HTTP connection itself fails.
pub async fn proxy_request(
    client: &reqwest::Client,
    target_base: &str,
    path: &str,
    auth: Option<(&str, &str)>,
    vault: Option<&crate::vault::Vault>,
    req: axum::extract::Request,
) -> Result<Response, String> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(e) => return Ok(json_error(StatusCode::BAD_REQUEST, &e.to_string())),
    };

    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let target_url = format!("{}{}{}", target_base.trim_end_matches('/'), path, query);

    let mut outbound = client.request(method, &target_url).body(body_bytes);
    for (name, value) in &headers {
        if !HOP_BY_HOP.contains(&name.as_str()) {
            outbound = outbound.header(name, value);
        }
    }

    if let Some((header_name, encrypted_key)) = auth {
        let v = match vault {
            Some(v) => v,
            None => {
                return Ok(json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "vault not initialized",
                ))
            }
        };
        let key = match v.decrypt(encrypted_key) {
            Ok(k) => k,
            Err(_) => {
                return Ok(json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "credential decryption failed",
                ))
            }
        };
        outbound = outbound.header(header_name, &key);
        // key drops here — never logged
    }

    match outbound.send().await {
        Err(e) => Err(format!("upstream unavailable: {}", e)),
        Ok(upstream) => {
            let status = upstream.status();
            let upstream_headers = upstream.headers().clone();
            let stream = upstream.bytes_stream();

            let mut builder = Response::builder().status(status);
            for (name, value) in &upstream_headers {
                if !HOP_BY_HOP.contains(&name.as_str()) {
                    builder = builder.header(name, value);
                }
            }
            Ok(builder
                .body(Body::from_stream(stream))
                .unwrap_or_else(|e| {
                    json_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
                }))
        }
    }
}

fn json_error(status: StatusCode, msg: &str) -> Response {
    let body = serde_json::to_string(&json!({ "error": msg }))
        .unwrap_or_else(|_| format!(r#"{{"error":"{}"}}"#, msg));
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{Config, GatewayConfig, RateLimitConfig},
        logger::RequestLogger,
        rate_limit::RateLimiter,
        routes::build_router,
        state::AppState,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn unknown_server_returns_404() {
        let rl_config = RateLimitConfig {
            enabled: false,
            requests_per_second: 10.0,
            burst: 20.0,
        };
        let state = Arc::new(AppState {
            config: Config {
                gateway: GatewayConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8080,
                    log_level: "error".to_string(),
                    rate_limit: rl_config.clone(),
                    db_path: None,
                },
                servers: vec![],
            },
            http_client: reqwest::Client::new(),
            vault: None,
            rate_limiter: Arc::new(RateLimiter::new(&rl_config)),
            logger: Arc::new(RequestLogger::new(":memory:").unwrap()),
        });

        let app = build_router(state);
        let req = Request::builder()
            .uri("/mcp/nonexistent/tools/list")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
