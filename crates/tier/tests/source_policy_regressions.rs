#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::json;
use tier::{
    ArgsSource, ConfigError, ConfigLoader, ConfigMetadata, EnvSource, FieldMetadata, MergeStrategy,
    SourceKind,
};

fn restricted(path: &str, kind: SourceKind) -> ConfigMetadata {
    ConfigMetadata::from_fields([FieldMetadata::new(path).deny_sources([kind])])
}

fn assert_denied(error: ConfigError, expected: &str, kind: SourceKind) {
    match error {
        ConfigError::SourcePolicyViolation { path, trace, .. } => {
            assert_eq!(path, expected);
            assert_eq!(trace.kind, kind);
        }
        other => panic!("expected source policy violation: {other:?}"),
    }
}

#[test]
fn env_object_writes_obey_descendant_source_policies() {
    let error = ConfigLoader::new(json!({"server":{"token":"default"}}))
        .metadata(restricted("server.token", SourceKind::Environment))
        .env(EnvSource::from_pairs([("APP_SERVER", r#"{"token":"env-write"}"#)]).prefix("APP"))
        .load()
        .unwrap_err();
    assert_denied(error, "server.token", SourceKind::Environment);
}

#[test]
fn cli_array_writes_obey_wildcard_source_policies() {
    let error = ConfigLoader::new(json!({"users":[]}))
        .metadata(restricted("users.*.token", SourceKind::Arguments))
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"users=[{"token":"cli-write"}]"#,
        ]))
        .load()
        .unwrap_err();
    assert_denied(error, "users.0.token", SourceKind::Arguments);
}

#[test]
fn custom_env_decoder_objects_obey_descendant_source_policies() {
    let error = ConfigLoader::new(json!({"server":{"token":"default"}}))
        .metadata(restricted("server.token", SourceKind::Environment))
        .env_decoder_with("server", |raw| Ok::<_, String>(json!({"token":raw})))
        .env(EnvSource::from_pairs([("APP_SERVER", "env-write")]).prefix("APP"))
        .load()
        .unwrap_err();
    assert_denied(error, "server.token", SourceKind::Environment);
}

#[test]
fn sparse_overrides_do_not_claim_untouched_array_elements() {
    let loaded = ConfigLoader::new(json!({"users":[{"token":"keep"},{"token":"old"}]}))
        .metadata(restricted("users.0.token", SourceKind::Arguments))
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"users[1]={"token":"new"}"#,
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.config()["users"][0]["token"], "keep");
    assert_eq!(loaded.config()["users"][1]["token"], "new");
}

#[test]
fn append_policies_check_destination_indices() {
    let metadata = ConfigMetadata::from_fields([
        FieldMetadata::new("users").merge_strategy(MergeStrategy::Append),
        FieldMetadata::new("users.0.token").deny_sources([SourceKind::Arguments]),
    ]);
    let loaded = ConfigLoader::new(json!({"users":[{"token":"keep"}]}))
        .metadata(metadata)
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"users=[{"token":"new"}]"#,
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.config()["users"][1]["token"], "new");
    assert_eq!(
        loaded.report().traces()["users.1.token"]
            .last()
            .unwrap()
            .source
            .kind,
        SourceKind::Arguments
    );
}

#[test]
fn structured_override_children_record_their_real_source() {
    let loaded = ConfigLoader::new(json!({"server":{"token":"default"}}))
        .env(EnvSource::from_pairs([("APP_SERVER", r#"{"token":"env-write"}"#)]).prefix("APP"))
        .load()
        .unwrap();
    let explanation = loaded.report().explain("server.token").unwrap();
    assert_eq!(explanation.steps.last().unwrap().source.name, "APP_SERVER");
    assert_eq!(explanation.steps.last().unwrap().value, "env-write");
}

#[test]
fn parent_replacements_cannot_delete_restricted_descendants() {
    for replacement in ["{}", "null", "42"] {
        let error = ConfigLoader::new(json!({"server":{"token":"protected"}}))
            .metadata(ConfigMetadata::from_fields([
                FieldMetadata::new("server").merge_strategy(MergeStrategy::Replace),
                FieldMetadata::new("server.token").deny_sources([SourceKind::Environment]),
            ]))
            .env(EnvSource::from_pairs([("APP_SERVER", replacement)]).prefix("APP"))
            .load()
            .unwrap_err();
        match &error {
            ConfigError::SourcePolicyViolation { trace, .. } => {
                assert_eq!(trace.name, "APP_SERVER")
            }
            other => panic!("expected source policy violation: {other:?}"),
        }
        assert_denied(error, "server.token", SourceKind::Environment);
    }
}

#[test]
fn array_truncation_cannot_delete_restricted_descendants() {
    let error = ConfigLoader::new(json!({"users":[{"token":"keep"},{"token":"protected"}]}))
        .metadata(restricted("users.1.token", SourceKind::Arguments))
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"users=[{"token":"new"}]"#,
        ]))
        .load()
        .unwrap_err();
    assert_denied(error, "users.1.token", SourceKind::Arguments);
}

#[test]
fn merging_empty_objects_does_not_claim_untouched_descendants() {
    let loaded = ConfigLoader::new(json!({"server":{"token":"protected"}}))
        .metadata(restricted("server.token", SourceKind::Environment))
        .env(EnvSource::from_pairs([("APP_SERVER", "{}")]).prefix("APP"))
        .load()
        .unwrap();
    assert_eq!(loaded.config()["server"]["token"], "protected");
}

#[cfg(feature = "derive")]
#[test]
fn typed_patch_object_writes_obey_descendant_source_policies() {
    #[derive(tier::TierPatch)]
    struct Patch {
        server: Option<serde_json::Value>,
    }
    let patch = Patch {
        server: Some(json!({"token":"patch-write"})),
    };
    let error = ConfigLoader::new(json!({"server":{"token":"default"}}))
        .metadata(restricted("server.token", SourceKind::Custom))
        .patch("custom", &patch)
        .unwrap()
        .load()
        .unwrap_err();
    assert_denied(error, "server.token", SourceKind::Custom);
}
