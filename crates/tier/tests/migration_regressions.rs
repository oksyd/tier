#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use tier::{
    ArgsSource, ConfigError, ConfigLoader, ConfigMigration, EnvSource, Layer,
    MigrationConflictPolicy, SourceKind,
};

#[derive(Debug, Serialize, Deserialize)]
struct VersionedNumber {
    version: u32,
    value: u32,
}

fn defaults() -> VersionedNumber {
    VersionedNumber {
        version: 1,
        value: 0,
    }
}

#[test]
fn config_versions_accept_bare_cli_and_environment_integers() {
    let cli = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .args(ArgsSource::from_args(["app", "--set", "version=1"]))
        .load()
        .unwrap();
    let env = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .env(EnvSource::from_pairs([("APP_VERSION", "1")]).prefix("APP"))
        .load()
        .unwrap();
    assert_eq!(cli.version, 1);
    assert_eq!(env.version, 1);
}

#[test]
fn bracket_version_paths_accept_bare_cli_integers() {
    let loaded = ConfigLoader::new(json!({"versions": [1]}))
        .config_version("versions[0]", 1)
        .args(ArgsSource::from_args(["app", "--set", "versions[0]=1"]))
        .load()
        .unwrap();
    assert_eq!(loaded.config()["versions"], json!([1]));
}

#[test]
fn config_versions_preserve_explicit_string_semantics() {
    let cli_error = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .env(EnvSource::from_pairs([("APP_VERSION", "1")]).prefix("APP"))
        .args(ArgsSource::from_args(["app", "--set", r#"version="1""#]))
        .load()
        .unwrap_err();
    let layer_error = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .layer(Layer::custom("typed", json!({"version": "1"})).unwrap())
        .load()
        .unwrap_err();
    assert!(matches!(
        cli_error,
        ConfigError::InvalidConfigVersion { .. }
    ));
    assert!(matches!(
        layer_error,
        ConfigError::InvalidConfigVersion { .. }
    ));
}

#[test]
fn coerced_config_versions_reject_invalid_ranges_and_future_versions() {
    for value in ["-1", "1.5", "4294967296", "18446744073709551616", "invalid"] {
        let error = ConfigLoader::new(defaults())
            .config_version("version", 1)
            .env(EnvSource::from_pairs([("APP_VERSION", value)]).prefix("APP"))
            .load()
            .unwrap_err();
        assert!(
            matches!(error, ConfigError::InvalidConfigVersion { .. }),
            "{value}: {error}"
        );
    }
    let error = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .args(ArgsSource::from_args(["app", "--set", "version=2"]))
        .load()
        .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::UnsupportedConfigVersion {
            found: 2,
            supported: 1,
            ..
        }
    ));
}

#[test]
fn chained_renames_preserve_scalar_coercion_and_original_source() {
    let loaded = ConfigLoader::new(defaults())
        .config_version("version", 1)
        .migration(ConfigMigration::rename("old", "intermediate", 1))
        .migration(ConfigMigration::rename("intermediate", "value", 1))
        .args(ArgsSource::from_args([
            "app",
            "--set",
            "version=0",
            "--set",
            "old=9",
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.value, 9);
    assert_eq!(loaded.report().migrations().len(), 2);
    let explanation = loaded.report().explain("value").unwrap();
    let step = explanation.steps.last().unwrap();
    assert_eq!(step.source.kind, SourceKind::Arguments);
    assert!(step.source.name.contains("old"));
}

#[test]
fn subtree_renames_preserve_environment_coercion() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Number {
        count: u32,
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        version: u32,
        current: Number,
    }
    let loaded = ConfigLoader::new(Config {
        version: 1,
        current: Number { count: 0 },
    })
    .config_version("version", 1)
    .migration(ConfigMigration::rename("old", "current", 1))
    .env(EnvSource::from_pairs([("APP_VERSION", "0"), ("APP_OLD__COUNT", "9")]).prefix("APP"))
    .load()
    .unwrap();
    assert_eq!(loaded.current.count, 9);
    assert_eq!(
        loaded
            .report()
            .explain("current.count")
            .unwrap()
            .steps
            .last()
            .unwrap()
            .source
            .name,
        "APP_OLD__COUNT"
    );
}

#[test]
fn repeated_rename_targets_reject_explicit_migrated_values() {
    for second_version in [1, 2] {
        let error = ConfigLoader::new(defaults())
            .config_version("version", second_version)
            .migration(ConfigMigration::rename("a", "value", 1))
            .migration(ConfigMigration::rename("b", "value", second_version))
            .layer(Layer::custom("first", json!({"version": 0, "a": 1})).unwrap())
            .layer(Layer::custom("second", json!({"b": 2})).unwrap())
            .load()
            .unwrap_err();
        let ConfigError::MigrationConflict {
            from_path,
            to_path,
            provenance,
        } = error
        else {
            panic!("expected migration conflict");
        };
        assert_eq!(from_path, "b");
        assert_eq!(to_path, "value");
        assert_eq!(provenance.unwrap().name, "first");
    }
}

#[test]
fn repeated_rename_targets_apply_keep_and_overwrite_policies() {
    for (policy, value, source) in [
        (MigrationConflictPolicy::KeepTarget, 1, "first"),
        (MigrationConflictPolicy::OverwriteTarget, 2, "second"),
    ] {
        let loaded = ConfigLoader::new(defaults())
            .config_version("version", 1)
            .migration(ConfigMigration::rename("a", "value", 1))
            .migration(ConfigMigration::rename_with_policy("b", "value", 1, policy))
            .layer(Layer::custom("first", json!({"version": 0, "a": 1})).unwrap())
            .layer(Layer::custom("second", json!({"b": 2})).unwrap())
            .load()
            .unwrap();
        assert_eq!(loaded.value, value);
        assert_eq!(
            loaded
                .report()
                .explain("value")
                .unwrap()
                .steps
                .last()
                .unwrap()
                .source
                .name,
            source
        );
    }
}

#[test]
fn replacing_an_object_detects_previously_migrated_descendant_sources() {
    let error = ConfigLoader::new(json!({"version": 1, "current": {"value": 0}}))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("old", "current.value", 1))
        .migration(ConfigMigration::rename("replacement", "current", 1))
        .layer(
            Layer::custom(
                "legacy",
                json!({"version": 0, "old": 9, "replacement": {"value": 2}}),
            )
            .unwrap(),
        )
        .load()
        .unwrap_err();
    assert!(
        matches!(error, ConfigError::MigrationConflict { to_path, .. } if to_path == "current")
    );
}

#[test]
fn rename_policies_keep_coercion_with_the_selected_value() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        version: u32,
        value: Option<String>,
    }
    for (policy, expected) in [
        (MigrationConflictPolicy::KeepTarget, Some("null")),
        (MigrationConflictPolicy::OverwriteTarget, None),
    ] {
        let loaded = ConfigLoader::new(Config {
            version: 1,
            value: None,
        })
        .config_version("version", 1)
        .migration(ConfigMigration::rename_with_policy(
            "old", "value", 1, policy,
        ))
        .env(EnvSource::from_pairs([("APP_VERSION", "0"), ("APP_OLD", "null")]).prefix("APP"))
        .args(ArgsSource::from_args(["app", "--set", r#"value="null""#]))
        .load()
        .unwrap();
        assert_eq!(loaded.value.as_deref(), expected);
    }
    let loaded = ConfigLoader::new(Config {
        version: 1,
        value: None,
    })
    .config_version("version", 1)
    .migration(ConfigMigration::rename_with_policy(
        "old",
        "value",
        1,
        MigrationConflictPolicy::OverwriteTarget,
    ))
    .env(EnvSource::from_pairs([("APP_VERSION", "0"), ("APP_VALUE", "null")]).prefix("APP"))
    .args(ArgsSource::from_args(["app", "--set", r#"old="null""#]))
    .load()
    .unwrap();
    assert_eq!(loaded.value.as_deref(), Some("null"));
}

#[test]
fn removing_array_elements_shifts_remaining_coercion_and_sources() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        version: u32,
        values: Vec<u32>,
    }
    let loaded = ConfigLoader::new(Config {
        version: 1,
        values: vec![0, 0, 0],
    })
    .config_version("version", 1)
    .migration(ConfigMigration::remove("values[0]", 1))
    .args(ArgsSource::from_args([
        "app",
        "--set",
        "version=0",
        "--set",
        "values[0]=obsolete",
        "--set",
        "values[1]=9",
        "--set",
        "values[2]=10",
    ]))
    .load()
    .unwrap();
    assert_eq!(loaded.values, [9, 10]);
    let trace = loaded.report().explain("values[0]").unwrap();
    assert!(
        trace
            .steps
            .last()
            .unwrap()
            .source
            .name
            .contains("values[1]")
    );
    let error = ConfigLoader::new(Config {
        version: 1,
        values: vec![0, 0],
    })
    .config_version("version", 1)
    .migration(ConfigMigration::remove("values[0]", 1))
    .args(ArgsSource::from_args([
        "app",
        "--set",
        "version=0",
        "--set",
        "values[0]=9",
        "--set",
        r#"values[1]="10""#,
    ]))
    .load()
    .unwrap_err();
    assert!(matches!(error, ConfigError::Deserialize { .. }));
}

#[test]
fn array_removal_preserves_explicit_sources_for_later_conflict_checks() {
    let error = ConfigLoader::new(json!({"version": 1, "values": [{"value": 0}]}))
        .config_version("version", 1)
        .migration(ConfigMigration::remove("values[0]", 1))
        .migration(ConfigMigration::rename("old", "values[0].value", 1))
        .layer(
            Layer::custom(
                "legacy",
                json!({"version": 0, "old": 9, "values": [{"value": 1}, {"value": 2}]}),
            )
            .unwrap(),
        )
        .load()
        .unwrap_err();
    assert!(
        matches!(error, ConfigError::MigrationConflict { to_path, provenance: Some(source), .. } if to_path == "values.0.value" && source.name == "legacy")
    );
}

#[test]
fn migrated_secret_values_are_redacted_at_their_new_trace_paths() {
    let loaded = ConfigLoader::new(json!({"version": 1, "value": "default"}))
        .config_version("version", 1)
        .secret_path("old")
        .secret_path("value")
        .migration(ConfigMigration::rename("old", "value", 1))
        .layer(Layer::custom("legacy", json!({"version": 0, "old": "private-credential"})).unwrap())
        .load()
        .unwrap();
    assert!(!format!("{:?}", loaded.report()).contains("private-credential"));
    assert_eq!(
        loaded
            .report()
            .explain("value")
            .unwrap()
            .steps
            .last()
            .unwrap()
            .value,
        "***redacted***"
    );
}
