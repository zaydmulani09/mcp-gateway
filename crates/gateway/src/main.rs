use gateway::{config, logger, rate_limit, routes, state, vault};

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path =
        env::var("MCPGW_CONFIG").unwrap_or_else(|_| "config/default.toml".to_string());
    let mut cfg = config::Config::load(&config_path)?;

    tracing_subscriber::fmt()
        .with_env_filter(&cfg.gateway.log_level)
        .init();

    let has_api_keys = cfg.servers.iter().any(|s| s.api_key.is_some());
    let v = if has_api_keys {
        let secret = env::var("MCPGW_MASTER_SECRET").unwrap_or_else(|_| {
            eprintln!(
                "error: MCPGW_MASTER_SECRET is required when any server has an api_key configured"
            );
            std::process::exit(1);
        });
        let v = vault::Vault::new(&secret);
        drop(secret);
        for server in &mut cfg.servers {
            if let Some(ref plaintext) = server.api_key {
                let encrypted = v.encrypt(plaintext);
                server.api_key = Some(encrypted);
            }
        }
        Some(v)
    } else {
        None
    };

    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(&cfg.gateway.rate_limit));

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let db_path = cfg
        .gateway
        .db_path
        .as_deref()
        .unwrap_or("mcpgw.db")
        .to_string();
    let request_logger = Arc::new(logger::RequestLogger::new(&db_path)?);

    let app_state = Arc::new(state::AppState {
        config: cfg,
        http_client,
        vault: v,
        rate_limiter,
        logger: request_logger,
    });

    let addr = format!(
        "{}:{}",
        app_state.config.gateway.host, app_state.config.gateway.port
    );
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("listening on {}", addr);

    let router = routes::build_router(Arc::clone(&app_state));
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutting down gracefully");
}
