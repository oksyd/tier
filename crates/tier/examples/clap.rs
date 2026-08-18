use clap::Parser;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    db: DbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbConfig {
    password: Secret<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_owned(),
                port: 3000,
            },
            db: DbConfig {
                password: Secret::new("default-secret".to_owned()),
            },
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = AppCli::parse();

    let loaded = cli
        .config
        .apply(ConfigLoader::new(AppConfig::default()).secret_path("db.password"))
        .load()?;

    if let Some(output) = cli.config.render(&loaded)? {
        println!("{output}");
    } else {
        println!("listening on {}:{}", loaded.server.host, loaded.server.port);
    }

    Ok(())
}
