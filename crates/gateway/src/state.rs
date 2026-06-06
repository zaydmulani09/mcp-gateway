use std::sync::Arc;

pub struct AppState {
    pub config: crate::config::Config,
    pub http_client: reqwest::Client,
    pub vault: Option<crate::vault::Vault>,
    pub rate_limiter: Arc<crate::rate_limit::RateLimiter>,
    pub logger: Arc<crate::logger::RequestLogger>,
}
