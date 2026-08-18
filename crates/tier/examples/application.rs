use std::fs;

use serde::{Deserialize, Serialize};
#[cfg(feature = "derive")]
use tier::TierConfig;
use tier::{ConfigLoader, EnvSource, Secret, ValidationErrors};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "derive", derive(TierConfig))]
struct AppConfig {
    server: ServerConfig,
    db: DbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "derive", derive(TierConfig))]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "derive", derive(TierConfig))]
struct DbConfig {
    url: String,
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
                url: "postgres://localhost/app".to_owned(),
                password: Secret::new("secret".to_owned()),
            },
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = tempfile::tempdir()?;
    fs::write(
        config_dir.path().join("default.toml"),
        r#"
[server]
host = "127.0.0.1"
port = 3000

[db]
url = "postgres://localhost/app"
password = "file-secret"
"#,
    )?;
    fs::write(
        config_dir.path().join("dev.toml"),
        r#"
[server]
host = "0.0.0.0"
"#,
    )?;

    let loader = ConfigLoader::new(AppConfig::default());
    #[cfg(feature = "derive")]
    let loader = loader.derive_metadata();
    #[cfg(not(feature = "derive"))]
    let loader = loader.secret_path("db.password");

    let env = EnvSource::from_pairs([("APP__SERVER__PORT", "8080")]).prefix("APP");
    let loaded = loader
        .file(config_dir.path().join("default.toml"))
        .optional_file(config_dir.path().join("{profile}.toml"))
        .env(env)
        .profile("dev")
        .validator("port-range", |config| {
            if config.server.port == 0 {
                return Err(ValidationErrors::from_message(
                    "server.port",
                    "port must be greater than zero",
                ));
            }
            Ok(())
        })
        .load()?;

    println!("{}", loaded.report().doctor());
    println!("{}", loaded.report().redacted_pretty_json());
    Ok(())
}
