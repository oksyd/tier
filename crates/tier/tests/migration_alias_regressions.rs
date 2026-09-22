#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::json;
use tier::{
    ConfigError, ConfigLoader, ConfigMetadata, ConfigMigration, EnvSource, FieldMetadata, Layer,
    MigrationConflictPolicy, SourceKind,
};

#[test]
fn aliased_migration_targets_enforce_canonical_source_policies() {
    let error = ConfigLoader::new(json!({"version": 1}))
        .metadata(ConfigMetadata::from_fields([FieldMetadata::new("token")
            .alias("credential")
            .deny_sources([SourceKind::Environment])]))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("old", "credential", 1))
        .env(EnvSource::from_pairs([("APP_VERSION", "0"), ("APP_OLD", "private")]).prefix("APP"))
        .load()
        .unwrap_err();
    assert!(matches!(
        error,
        ConfigError::SourcePolicyViolation { path, trace, .. }
            if path == "token" && trace.kind == SourceKind::Environment
    ));
}

#[test]
fn aliased_migration_targets_respect_conflict_policies() {
    for policy in [
        MigrationConflictPolicy::Error,
        MigrationConflictPolicy::KeepTarget,
        MigrationConflictPolicy::OverwriteTarget,
    ] {
        let result = ConfigLoader::new(json!({"version": 1, "value": 0}))
            .metadata(ConfigMetadata::from_fields([
                FieldMetadata::new("value").alias("renamed")
            ]))
            .config_version("version", 1)
            .migration(ConfigMigration::rename_with_policy(
                "old", "renamed", 1, policy,
            ))
            .layer(Layer::custom("explicit", json!({"version": 0, "old": 9, "value": 2})).unwrap())
            .load();
        match policy {
            MigrationConflictPolicy::Error => assert!(
                matches!(result, Err(ConfigError::MigrationConflict { to_path, .. }) if to_path == "value")
            ),
            MigrationConflictPolicy::KeepTarget => assert_eq!(result.unwrap().config()["value"], 2),
            MigrationConflictPolicy::OverwriteTarget => {
                assert_eq!(result.unwrap().config()["value"], 9)
            }
        }
    }
}

#[test]
fn aliased_version_paths_check_the_effective_version() {
    let error = ConfigLoader::new(json!({"version": 1}))
        .metadata(ConfigMetadata::from_fields([
            FieldMetadata::new("version").alias("revision")
        ]))
        .config_version("revision", 1)
        .layer(Layer::custom("future", json!({"revision": 2})).unwrap())
        .load()
        .unwrap_err();
    assert!(
        matches!(error, ConfigError::UnsupportedConfigVersion { path, found: 2, supported: 1 } if path == "version")
    );
}

#[test]
fn migration_sources_resolve_aliases_before_renaming_and_removing() {
    let loaded = ConfigLoader::new(json!({"version": 1}))
        .metadata(ConfigMetadata::from_fields([
            FieldMetadata::new("old").alias("legacy"),
            FieldMetadata::new("obsolete").alias("deprecated"),
        ]))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("legacy", "current", 1))
        .migration(ConfigMigration::remove("deprecated", 1))
        .layer(
            Layer::custom(
                "legacy input",
                json!({"version": 0, "legacy": 7, "deprecated": true}),
            )
            .unwrap(),
        )
        .load()
        .unwrap();
    assert_eq!(loaded.config(), &json!({"version": 1, "current": 7}));
    assert_eq!(loaded.report().migrations().len(), 2);
}

#[test]
fn an_alias_already_resolved_to_the_migration_target_is_a_noop() {
    let loaded = ConfigLoader::new(json!({"version": 1, "current": 0}))
        .metadata(ConfigMetadata::from_fields([
            FieldMetadata::new("current").alias("legacy")
        ]))
        .config_version("version", 1)
        .migration(ConfigMigration::rename("legacy", "current", 1))
        .layer(Layer::custom("legacy input", json!({"version": 0, "legacy": 7})).unwrap())
        .load()
        .unwrap();
    assert_eq!(loaded.config(), &json!({"version": 1, "current": 7}));
    assert!(loaded.report().migrations().is_empty());
}

#[test]
fn migration_created_parents_cannot_remove_source_restricted_descendants() {
    for staged_default in [None, Some(json!({}))] {
        let mut defaults =
            json!({"version": 1, "current": {"token": "protected"}, "legacy_default": "allowed"});
        if let Some(staged) = staged_default {
            defaults["staged"] = staged;
        }
        let error = ConfigLoader::new(defaults)
            .metadata(ConfigMetadata::from_fields([FieldMetadata::new(
                "current.token",
            )
            .deny_sources([SourceKind::Environment])]))
            .config_version("version", 1)
            .migration(ConfigMigration::rename("legacy_default", "staged.a", 1))
            .migration(ConfigMigration::rename("old", "staged.z", 1))
            .migration(ConfigMigration::rename("staged", "current", 1))
            .env(EnvSource::from_pairs([("APP_VERSION", "0"), ("APP_OLD", "9")]).prefix("APP"))
            .load()
            .unwrap_err();
        assert!(
            matches!(error, ConfigError::SourcePolicyViolation { path, trace, .. }
            if path == "current.token" && trace.kind == SourceKind::Environment)
        );
    }
}
