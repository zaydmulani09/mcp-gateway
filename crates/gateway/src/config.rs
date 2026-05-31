use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: f64,
    pub burst: f64,
}

#[derive(Deserialize)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub rate_limit: RateLimitConfig,
    pub db_path: Option<String>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub api_key: Option<String>,
    pub auth_header: Option<String>,
}

#[derive(Deserialize)]
pub struct Config {
    pub gateway: GatewayConfig,
    pub servers: Vec<ServerConfig>,
}

impl Config {
    pub fn load(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn find_server(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }
}
