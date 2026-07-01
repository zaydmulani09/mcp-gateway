use gateway::{
    config::{Config, GatewayConfig, RateLimitConfig, ServerConfig},
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

fn make_state(servers: Vec<ServerConfig>, rl_enabled: bool, burst: f64) -> Arc<AppState> {
    let rl_config = RateLimitConfig {
        enabled: rl_enabled,
        requests_per_second: 10.0,
        burst,
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
async fn health_check_returns_ok() {
    let state = make_state(vec![], false, 20.0);
    let app = build_router(state);
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("ok"));
}

#[tokio::test]
async fn unknown_server_404() {
    let state = make_state(vec![], false, 20.0);
    let app = build_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/mcp/ghost/tools/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn disabled_server_503() {
    let state = make_state(
        vec![ServerConfig {
            name: "disabled-server".to_string(),
            url: "http://localhost:3001".to_string(),
            enabled: false,
            api_key: None,
            auth_header: None,
        }],
        false,
        20.0,
    );
    let app = build_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/mcp/disabled-server/tools/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn rate_limit_429() {
    let state = make_state(
        vec![ServerConfig {
            name: "test-server".to_string(),
            url: "http://localhost:9999".to_string(),
            enabled: true,
            api_key: None,
            auth_header: None,
        }],
        true,
        2.0,
    );

    // exhaust burst=2 tokens so next HTTP request gets 429
    state.rate_limiter.check("127.0.0.1", "test-server");
    state.rate_limiter.check("127.0.0.1", "test-server");

    let app = build_router(Arc::clone(&state));
    let req = Request::builder()
        .method("POST")
        .uri("/mcp/test-server/tools/list")
        .header("x-forwarded-for", "127.0.0.1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn stats_endpoint_returns_json_array() {
    let state = make_state(vec![], false, 20.0);
    let app = build_router(state);
    let req = Request::builder()
        .uri("/stats")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(value.is_array());
}

#[tokio::test]
async fn missing_server_name_400() {
    let state = make_state(vec![], false, 20.0);
    let app = build_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/mcp//tools/list")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
