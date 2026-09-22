#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
use serde::{Deserialize, Serialize};
use tier::{ArgsSource, ConfigLoader};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: u16,
}

#[test]
fn argument_terminator_keeps_application_arguments_out_of_configuration() {
    let loaded = ConfigLoader::new(Config { port: 3000 })
        .args(ArgsSource::from_args([
            "app",
            "--set",
            "port=4000",
            "--",
            "--config",
            "nonexistent.toml",
            "--set",
            "port=9000",
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.port, 4000);
}

#[cfg(unix)]
#[test]
fn non_unicode_application_arguments_after_terminator_are_not_parsed() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let loaded = ConfigLoader::new(Config { port: 3000 })
        .args(ArgsSource::from_os_args([
            OsString::from("app"),
            OsString::from("--"),
            OsString::from_vec(vec![0xff]),
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.port, 3000);
}

#[cfg(all(unix, feature = "toml"))]
#[test]
fn config_file_arguments_preserve_non_unicode_paths() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join(OsString::from_vec(b"config-\xff.toml".to_vec()));
    std::fs::write(&path, "port = 4000").unwrap();
    let loaded = ConfigLoader::new(Config { port: 3000 })
        .args(ArgsSource::from_os_args([
            OsString::from("app"),
            OsString::from("--config"),
            path.into_os_string(),
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.port, 4000);
}

#[cfg(feature = "clap")]
mod cli {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        config: tier::TierCli,
    }

    #[cfg(all(unix, feature = "toml"))]
    #[test]
    fn clap_to_args_source_preserves_non_unicode_paths() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join(OsString::from_vec(b"config-\xff.toml".to_vec()));
        std::fs::write(&path, "port = 4000").unwrap();
        let cli = Cli::parse_from([
            OsString::from("app"),
            OsString::from("--config"),
            path.into_os_string(),
        ]);
        let loaded = cli
            .config
            .apply(ConfigLoader::new(Config { port: 3000 }))
            .load()
            .unwrap();
        assert_eq!(loaded.port, 4000);
    }

    #[cfg(feature = "schema")]
    #[test]
    fn schema_commands_work_before_required_configuration_exists() {
        #[derive(Debug, Deserialize, schemars::JsonSchema)]
        #[allow(dead_code)]
        struct Required {
            token: String,
        }
        impl tier::TierMetadata for Required {
            fn metadata() -> tier::ConfigMetadata {
                tier::ConfigMetadata::new()
            }
        }
        assert!(
            ConfigLoader::<Required>::from_value(serde_json::json!({}))
                .load()
                .is_err()
        );
        for command in [
            "--print-config-schema",
            "--print-env-docs",
            "--print-config-example",
        ] {
            let cli = Cli::parse_from(["app", command]);
            let output = cli.config.render_schema::<Required>().unwrap();
            assert!(output.contains("token"));
        }
        let cli = Cli::parse_from(["app", "--print-config"]);
        assert!(cli.config.render_schema::<Required>().is_none());
    }
}
