use clap::{Parser, Subcommand};
use serde_json::Value;
use std::process;

#[derive(Parser)]
#[command(name = "mcpgw", version = "0.1.0", about = "MCP Gateway CLI")]
struct Cli {
    #[arg(long, default_value = "http://localhost:8080", env = "MCPGW_URL")]
    gateway: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server {
        #[command(subcommand)]
        cmd: ServerCmd,
    },
    Logs {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    Stats,
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(Subcommand)]
enum ServerCmd {
    List,
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    Show,
}

fn get_json(url: &str) -> Result<Value, String> {
    reqwest::blocking::get(url)
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { cmd } => match cmd {
            ServerCmd::List => {
                println!("Servers are configured in config/default.toml");
            }
            ServerCmd::Add { name, url, api_key } => {
                println!("Add to config/default.toml:\n");
                println!("[[servers]]");
                println!("name = {:?}", name);
                println!("url = {:?}", url);
                println!("enabled = true");
                if let Some(key) = api_key {
                    println!("api_key = {:?}", key);
                    println!("auth_header = \"Authorization\"");
                }
            }
        },
        Commands::Logs { limit } => {
            let url = format!("{}/stats", cli.gateway);
            match get_json(&url) {
                Ok(Value::Array(entries)) => {
                    for entry in entries.iter().take(limit as usize) {
                        let ts = entry.get("ts").and_then(|v| v.as_str()).unwrap_or("-");
                        let method =
                            entry.get("method").and_then(|v| v.as_str()).unwrap_or("-");
                        let path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("-");
                        let status = entry
                            .get("status")
                            .and_then(|v| v.as_i64())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "-".to_string());
                        let latency = entry
                            .get("latency_ms")
                            .and_then(|v| v.as_i64())
                            .map(|l| format!("{}ms", l))
                            .unwrap_or_else(|| "-".to_string());
                        println!("{ts}  {method}  {path}  {status}  {latency}");
                    }
                }
                Ok(_) => eprintln!("unexpected response format"),
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::Stats => {
            let url = format!("{}/stats", cli.gateway);
            match get_json(&url) {
                Ok(entries) => {
                    if let Some(arr) = entries.as_array() {
                        let errors = arr
                            .iter()
                            .filter(|e| e.get("error").and_then(|v| v.as_str()).is_some())
                            .count();
                        println!("total requests : {}", arr.len());
                        println!("errors         : {errors}");
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Show => {
                let path = std::env::var("MCPGW_CONFIG")
                    .unwrap_or_else(|_| "config/default.toml".to_string());
                match std::fs::read_to_string(&path) {
                    Ok(content) => print!("{content}"),
                    Err(e) => eprintln!("error reading {path}: {e}"),
                }
            }
        },
    }
}
