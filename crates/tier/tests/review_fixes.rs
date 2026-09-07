#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used, dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use tier::{
    ArgsSource, ConfigError, ConfigLoader, ConfigMetadata, ConfigMigration, EnvDecoder, EnvSource,
    FieldMetadata, SourceKind,
};

#[test]
fn migrations_preserve_secret_values_in_final_and_historical_reports() {
    for secret in ["old.token", "new.token"] {
        let loaded = ConfigLoader::new(
            json!({"version":0,"old":{"token":"private-credential","public":"keep"}}),
        )
        .secret_path(secret)
        .config_version("version", 2)
        .migration(ConfigMigration::rename("old", "new", 1))
        .migration(ConfigMigration::rename("new", "current", 2))
        .normalizer("no-op", |_| Ok::<_, String>(()))
        .load()
        .unwrap();
        assert_eq!(loaded.config()["current"]["token"], "private-credential");
        assert_eq!(
            loaded.report().redacted_final_value()["current"]["public"],
            "keep"
        );
        assert!(!format!("{:?}", loaded.report()).contains("private-credential"));
        assert!(
            !loaded
                .report()
                .audit_json_pretty()
                .contains("private-credential")
        );
    }
}

#[test]
fn removing_array_entries_preserves_shifted_secrets() {
    let loaded = ConfigLoader::new(
        json!({"version":0,"items":[{"token":"public"},{"token":"private-token"}]}),
    )
    .secret_path("items[1].token")
    .config_version("version", 1)
    .migration(ConfigMigration::remove("items[0]", 1))
    .load()
    .unwrap();
    assert_eq!(
        loaded.report().redacted_final_value()["items"][0]["token"],
        "***redacted***"
    );
    assert!(!format!("{:?}", loaded.report()).contains("private-token"));
}

#[test]
fn renamed_environment_values_obey_target_source_policies() {
    let error = ConfigLoader::new(json!({"version":0,"token":"default"}))
        .metadata(ConfigMetadata::from_fields([
            FieldMetadata::new("token").deny_sources([SourceKind::Environment])
        ]))
        .env(EnvSource::from_pairs([("APP_OLD", "override")]).prefix("APP"))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("old", "token", 1))
        .load()
        .unwrap_err();
    assert!(
        matches!(error,ConfigError::SourcePolicyViolation{path,trace,..} if path=="token" && trace.name=="APP_OLD")
    );
}

#[test]
fn renamed_objects_cannot_remove_protected_target_children() {
    let error = ConfigLoader::new(json!({"version":0,"target":{"token":"protected"}}))
        .metadata(ConfigMetadata::from_fields([FieldMetadata::new(
            "target.token",
        )
        .deny_sources([SourceKind::Environment])]))
        .env(EnvSource::from_pairs([("APP_OLD", "{}")]).prefix("APP"))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("old", "target", 1))
        .load()
        .unwrap_err();
    assert!(matches!(error,ConfigError::SourcePolicyViolation{path,..} if path=="target.token"));
}

#[test]
fn env_decoder_errors_hide_sensitive_input_but_preserve_public_diagnostics() {
    for secret in [true, false] {
        let loader = ConfigLoader::new(json!({"credentials":{}}))
            .env_decoder("credentials", EnvDecoder::KeyValueMap)
            .env(EnvSource::from_pairs([("APP_CREDENTIALS", "private-bad-entry")]).prefix("APP"));
        let error = if secret {
            loader.secret_path("credentials")
        } else {
            loader
        }
        .load()
        .unwrap_err();
        assert_eq!(format!("{error:?}").contains("private-bad-entry"), !secret);
        assert!(error.to_string().contains("APP_CREDENTIALS"));
    }
}

#[cfg(feature = "toml")]
#[test]
fn file_parse_errors_hide_source_lines_when_secrets_are_registered() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "token = private-file-secret\n").unwrap();
    for secret in [true, false] {
        let loader = ConfigLoader::new(json!({"token":""})).file(&path);
        let error = if secret {
            loader.secret_path("token")
        } else {
            loader
        }
        .load()
        .unwrap_err();
        assert_eq!(
            format!("{error:?}").contains("private-file-secret"),
            !secret
        );
        assert!(matches!(
            error,
            ConfigError::ParseFile {
                location: Some(_),
                ..
            }
        ));
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Numbers {
    name: String,
    count: u32,
    second: u32,
    text: String,
    enabled: bool,
}
#[derive(Debug, Serialize, Deserialize)]
struct Flat {
    #[serde(flatten)]
    inner: Numbers,
}

#[test]
fn flatten_coerces_only_rejected_values_even_when_strings_have_identical_contents() {
    let loaded = ConfigLoader::<Flat>::from_value(json!({}))
        .env(
            EnvSource::from_pairs([
                ("APP_NAME", "7"),
                ("APP_COUNT", "7"),
                ("APP_SECOND", "7"),
                ("APP_TEXT", "true"),
                ("APP_ENABLED", "true"),
            ])
            .prefix("APP"),
        )
        .load()
        .unwrap();
    assert_eq!(loaded.config().inner.name, "7");
    assert_eq!(loaded.config().inner.count, 7);
    assert_eq!(loaded.config().inner.second, 7);
    assert_eq!(loaded.config().inner.text, "true");
    assert!(loaded.config().inner.enabled);
    assert_eq!(
        loaded.report().redacted_final_value(),
        &json!({"name":"7","count":7,"second":7,"text":"true","enabled":true})
    );
}

#[test]
fn flatten_keeps_explicit_json_strings_strict() {
    let error = ConfigLoader::<Flat>::from_value(
        json!({"name":"7","count":0,"second":0,"text":"true","enabled":false}),
    )
    .args(ArgsSource::from_args(["app", "--set", r#"count="7""#]))
    .load()
    .unwrap_err();
    assert!(matches!(error, ConfigError::Deserialize { .. }));
}

#[test]
fn normalizer_replacements_do_not_inherit_environment_coercion_permission() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        count: u32,
    }
    for with_env in [true, false] {
        let mut loader = ConfigLoader::new(Config { count: 0 });
        if with_env {
            loader = loader.env(EnvSource::from_pairs([("APP_COUNT", "1")]).prefix("APP"));
        }
        let error = loader
            .normalizer("replace", |value| {
                value["count"] = json!("7");
                Ok::<_, String>(())
            })
            .load()
            .unwrap_err();
        assert!(matches!(error,ConfigError::Deserialize{path,..} if path=="count"));
    }
}

#[test]
fn no_op_normalizers_preserve_environment_coercion() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        count: u32,
    }
    let loaded = ConfigLoader::new(Config { count: 0 })
        .env(EnvSource::from_pairs([("APP_COUNT", "7")]).prefix("APP"))
        .normalizer("no-op", |_| Ok::<_, String>(()))
        .load()
        .unwrap();
    assert_eq!(loaded.config().count, 7);
}

#[cfg(feature = "derive")]
mod derive {
    use super::*;
    use tier::TierConfig;
    #[derive(Debug, Serialize, Deserialize, TierConfig)]
    struct Directional {
        #[serde(rename(serialize = "out", deserialize = "input"))]
        name: String,
    }
    #[test]
    fn directional_renames_accept_both_input_and_serialized_defaults() {
        for value in [json!({"input":"value"}), json!({"out":"value"})] {
            let loaded = ConfigLoader::<Directional>::from_value(value)
                .derive_metadata()
                .load()
                .unwrap();
            assert_eq!(loaded.config().name, "value");
            assert_eq!(
                loaded.report().redacted_final_value(),
                &json!({"input":"value"})
            );
        }
        let loaded = ConfigLoader::new(Directional {
            name: "default".into(),
        })
        .derive_metadata()
        .load()
        .unwrap();
        assert_eq!(loaded.config().name, "default");
    }
    #[derive(Debug, Serialize, Deserialize, TierConfig)]
    #[serde(rename_all(serialize = "UPPERCASE", deserialize = "lowercase"))]
    enum Choice {
        Old { a: u32 },
        New { b: u32 },
    }
    #[test]
    fn root_enum_defaults_load_and_switch_variants_without_stale_fields() {
        let defaults = ConfigLoader::new(Choice::Old { a: 1 })
            .derive_metadata()
            .load()
            .unwrap();
        assert!(matches!(defaults.config(), Choice::Old { a: 1 }));
        let loaded = ConfigLoader::new(Choice::Old { a: 1 })
            .derive_metadata()
            .args(ArgsSource::from_args(["app", "--set", r#"NEW={"b":2}"#]))
            .load()
            .unwrap();
        assert!(matches!(loaded.config(), Choice::New { b: 2 }));
        assert_eq!(
            loaded.report().redacted_final_value(),
            &json!({"new":{"b":2}})
        );
    }
}

#[cfg(feature = "schema")]
mod schema {
    use super::*;
    #[derive(Debug, Deserialize, schemars::JsonSchema)]
    struct Node {
        token: tier::Secret<String>,
        label: String,
        next: Option<Box<Node>>,
    }
    fn nested() -> serde_json::Value {
        let mut value = serde_json::Value::Null;
        for _ in 0..5 {
            value = json!({"token":"private-recursive-token","label":"public-label","next":value});
        }
        value
    }
    #[test]
    fn recursive_schema_secrets_are_redacted_at_every_depth_without_hiding_public_fields() {
        let loaded = ConfigLoader::<Node>::from_value(nested())
            .discover_secret_paths_from_schema()
            .load()
            .unwrap();
        assert!(!format!("{:?}", loaded.report()).contains("private-recursive-token"));
        assert_eq!(
            loaded.report().redacted_final_value()["next"]["next"]["label"],
            "public-label"
        );
        assert_eq!(
            loaded.report().redacted_final_value()["next"]["next"]["token"],
            "***redacted***"
        );
    }
    #[test]
    fn recursive_secrets_created_by_normalizers_are_redacted_in_traces() {
        let loaded =
            ConfigLoader::<Node>::from_value(json!({"token":"safe","label":"public","next":null}))
                .discover_secret_paths_from_schema()
                .normalizer("extend", |value| {
                    value["next"] = nested();
                    Ok::<_, String>(())
                })
                .load()
                .unwrap();
        assert!(!format!("{:?}", loaded.report()).contains("private-recursive-token"));
    }
}

#[test]
fn env_decoder_errors_respect_secret_aliases() {
    let error = ConfigLoader::new(json!({"credentials": {}}))
        .metadata(ConfigMetadata::from_fields([FieldMetadata::new(
            "credentials",
        )
        .alias("legacy")]))
        .secret_path("legacy")
        .env_decoder("credentials", EnvDecoder::KeyValueMap)
        .env(EnvSource::from_pairs([("APP_CREDENTIALS", "private-alias-entry")]).prefix("APP"))
        .load()
        .unwrap_err();
    assert!(!format!("{error:?}").contains("private-alias-entry"));
}

#[test]
fn array_migrations_cannot_shift_environment_values_into_restricted_paths() {
    let error =
        ConfigLoader::new(json!({"version":0,"items":[{"token":"protected"},{"token":"old"}]}))
            .metadata(ConfigMetadata::from_fields([FieldMetadata::new(
                "items.0.token",
            )
            .deny_sources([SourceKind::Environment])]))
            .env(EnvSource::from_pairs([("APP_ITEMS__1__TOKEN", "override")]).prefix("APP"))
            .config_version("version", 1)
            .migration(ConfigMigration::remove("items[0]", 1))
            .load()
            .unwrap_err();
    assert!(matches!(error,ConfigError::SourcePolicyViolation{path,..} if path=="items.0.token"));
}
