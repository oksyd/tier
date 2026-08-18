#![cfg(feature = "clap")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use clap::{CommandFactory, Parser};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use tier::{
    ConfigLoader, ConfigMetadata, FieldMetadata, Secret, TierCli, TierCliCommand, TierMetadata,
};

#[derive(Debug, Parser)]
struct AppCli {
    #[command(flatten)]
    config: TierCli,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct CliConfig {
    server: CliServer,
    db: CliDb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct CliServer {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
struct CliDb {
    password: Secret<String>,
}

impl TierMetadata for CliConfig {
    fn metadata() -> ConfigMetadata {
        ConfigMetadata::from_fields([
            FieldMetadata::new("server.host")
                .env("APP_SERVER_HOSTNAME")
                .doc("Address exposed by the CLI application"),
            FieldMetadata::new("db.password").secret(),
        ])
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            server: CliServer {
                host: "127.0.0.1".to_owned(),
                port: 3000,
            },
            db: CliDb {
                password: Secret::new("clap-secret".to_owned()),
            },
        }
    }
}

#[test]
fn clap_cli_parses_and_applies_loader_overrides() {
    let cli = AppCli::parse_from([
        "tier-app",
        "--profile",
        "prod",
        "--set",
        "server.port=9001",
        "--set",
        "server.host=\"0.0.0.0\"",
        "--print-config",
    ]);

    assert_eq!(cli.config.command(), TierCliCommand::PrintConfig);

    let loaded = cli
        .config
        .apply(ConfigLoader::new(CliConfig::default()).secret_path("db.password"))
        .load()
        .expect("config loads");

    assert_eq!(loaded.server.port, 9001);
    assert_eq!(loaded.server.host, "0.0.0.0");

    let output = cli
        .config
        .render(&loaded)
        .expect("render succeeds")
        .expect("print output");
    assert!(output.contains("***redacted***"));
    assert!(!output.contains("clap-secret"));
}

#[test]
fn clap_cli_help_keeps_tier_docs_out_of_runtime_help() {
    let help = AppCli::command().render_long_help().to_string();

    assert!(help.contains("Usage:"));
    assert!(help.contains("--set <KEY=VALUE>"));
    assert!(!help.contains("Reusable `clap` flag group"));
    assert!(!help.contains("```ignore"));
}

#[test]
fn clap_cli_can_render_explain_output() {
    let cli = AppCli::parse_from(["tier-app", "--explain-config", "server.port"]);
    assert_eq!(
        cli.config.command(),
        TierCliCommand::ExplainConfig {
            path: "server.port".to_owned()
        }
    );

    let loaded = cli
        .config
        .apply(ConfigLoader::new(CliConfig::default()).secret_path("db.password"))
        .load()
        .expect("config loads");

    let output = cli
        .config
        .render(&loaded)
        .expect("render succeeds")
        .expect("explain output");
    assert!(output.contains("server.port = 3000"));
}

#[cfg(feature = "schema")]
#[test]
fn clap_cli_can_render_schema_and_env_docs() {
    let schema_cli = AppCli::parse_from(["tier-app", "--print-config-schema"]);
    assert_eq!(
        schema_cli.config.command(),
        TierCliCommand::PrintConfigSchema
    );

    let env_cli = AppCli::parse_from(["tier-app", "--print-env-docs", "--env-prefix", "APP"]);
    assert_eq!(env_cli.config.command(), TierCliCommand::PrintEnvDocs);

    let example_cli = AppCli::parse_from(["tier-app", "--print-config-example"]);
    assert_eq!(
        example_cli.config.command(),
        TierCliCommand::PrintConfigExample
    );

    let loaded = ConfigLoader::new(CliConfig::default())
        .secret_path("db.password")
        .load()
        .expect("config loads");

    let generic_render_error = schema_cli
        .config
        .render(&loaded)
        .expect_err("schema commands require schema-aware rendering");
    assert!(
        generic_render_error
            .to_string()
            .contains("render_with_schema")
    );

    let schema_output = schema_cli
        .config
        .render_with_schema(&loaded)
        .expect("schema render succeeds")
        .expect("schema output");
    assert!(schema_output.contains("\"type\": \"object\""));
    assert!(schema_output.contains("\"x-tier-env\""));

    let env_output = env_cli
        .config
        .render_with_schema(&loaded)
        .expect("env doc render succeeds")
        .expect("env docs");
    assert!(env_output.contains("APP_SERVER_HOSTNAME"));
    assert!(env_output.contains("APP__DB__PASSWORD"));

    let example_output = example_cli
        .config
        .render_with_schema(&loaded)
        .expect("example render succeeds")
        .expect("example config");
    #[cfg(feature = "toml")]
    assert!(example_output.contains("[server]"));
    #[cfg(not(feature = "toml"))]
    assert!(example_output.contains("\"server\""));
    assert!(example_output.contains("<secret>"));
}
