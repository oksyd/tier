#![allow(
    dead_code,
    reason = "configuration fields are consumed through serde and diagnostics"
)]

use serde::Deserialize;
use tier::{ConfigLoader, ConfigMetadata, EnvSource, FieldMetadata, Secret};

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    db: DbConfig,
    tls: TlsConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct DbConfig {
    url: String,
    password: Secret<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TlsConfig {
    enabled: bool,
    cert: Option<String>,
    key: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = ConfigMetadata::from_fields([
        FieldMetadata::new("db.url")
            .env("DATABASE_URL")
            .doc("Primary database connection string"),
        FieldMetadata::new("db.password").secret().non_empty(),
        FieldMetadata::new("tls.cert").absolute_path(),
        FieldMetadata::new("tls.key").absolute_path(),
    ])
    .required_if("tls.enabled", true, ["tls.cert", "tls.key"]);

    let env = EnvSource::from_pairs([
        ("DATABASE_URL", "\"postgres://env/app\""),
        ("APP__DB__PASSWORD", "\"rotated-secret\""),
    ])
    .prefix("APP");

    let loaded = ConfigLoader::<AppConfig>::from_value(serde_json::json!({
        "db": {
            "url": "postgres://localhost/app",
            "password": "default-secret"
        },
        "tls": {
            "enabled": true,
            "cert": "/etc/tier/tls.crt",
            "key": "/etc/tier/tls.key"
        }
    }))
    .metadata(metadata)
    .env(env)
    .load()?;

    println!("{}", loaded.report().redacted_pretty_json());
    let explanation = loaded
        .report()
        .explain("db.password")
        .ok_or_else(|| std::io::Error::other("db.password explanation missing"))?;
    println!("{explanation}");
    Ok(())
}
