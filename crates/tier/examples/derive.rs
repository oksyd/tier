#![allow(
    dead_code,
    reason = "configuration fields are consumed through serde and derived metadata"
)]

use serde::Deserialize;
use tier::{ArgsSource, ConfigLoader, EnvSource, Secret, TierConfig};

#[derive(Debug, Clone, Deserialize, TierConfig)]
#[tier(exactly_one_of("listener.port", "listener.unix_socket"))]
struct AppConfig {
    #[tier(doc = "Logical service name", non_empty, min_length = 3)]
    service_name: String,
    listener: ListenerConfig,
    db: DbConfig,
}

#[derive(Debug, Clone, Deserialize, TierConfig)]
struct ListenerConfig {
    #[tier(min = 1, max = 65535)]
    port: Option<u16>,
    unix_socket: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TierConfig)]
struct DbConfig {
    #[tier(env = "DATABASE_URL")]
    url: String,
    password: Secret<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = EnvSource::from_pairs([("DATABASE_URL", "\"postgres://env/app\"")]);
    let args = ArgsSource::from_args(["app", "--set", r#"db.password="rotated-secret""#]);

    let loaded = ConfigLoader::<AppConfig>::from_value(serde_json::json!({
        "service_name": "tier-api",
        "listener": { "port": 3000, "unix_socket": null },
        "db": {
            "url": "postgres://localhost/app",
            "password": "default-secret"
        }
    }))
    .derive_metadata()
    .env(env)
    .args(args)
    .load()?;

    println!("{}", loaded.report().doctor());
    println!("{}", loaded.report().redacted_pretty_json());
    Ok(())
}
