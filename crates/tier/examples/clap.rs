#![allow(
    dead_code,
    reason = "configuration fields are consumed through serde and CLI diagnostics"
)]

use clap::Parser;
use serde::Deserialize;
use tier::{ConfigLoader, Secret, TierCli};

#[derive(Debug, Parser)]
#[command(
    name = "tier-app",
    bin_name = "tier-app",
    about = "Example CLI using tier configuration flags"
)]
struct AppCli {
    #[command(flatten)]
    config: TierCli,
}

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    db: DbConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct DbConfig {
    password: Secret<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = AppCli::parse();

    let loaded = cli
        .config
        .apply(
            ConfigLoader::<AppConfig>::from_value(serde_json::json!({
                "server": { "host": "127.0.0.1", "port": 3000 },
                "db": { "password": "default-secret" }
            }))
            .secret_path("db.password"),
        )
        .load()?;

    if let Some(output) = cli.config.render(&loaded)? {
        println!("{output}");
    } else {
        println!("listening on {}:{}", loaded.server.host, loaded.server.port);
    }

    Ok(())
}
