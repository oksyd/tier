#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::json;
use tier::{ConfigLoader, EnvDecoder, EnvSource, FieldMetadata, Layer};

#[test]
fn historical_secrets_remain_hidden_when_objects_become_arrays() {
    for name in ["old", "00"] {
        let loaded = ConfigLoader::new(json!({"items": {name: "private-historical-value"}}))
            .secret_path(format!("items.{name}"))
            .layer(Layer::custom("replace", json!({"items": ["new"]})).unwrap())
            .load()
            .unwrap();
        assert!(!format!("{:?}", loaded.report()).contains("private-historical-value"));
        assert!(
            !loaded
                .report()
                .audit_json_pretty()
                .contains("private-historical-value")
        );
    }
}

#[test]
fn secret_array_paths_can_be_introduced_by_later_layers() {
    let loaded = ConfigLoader::new(json!({"items": {"old": "public"}}))
        .secret_path("items[0]")
        .layer(Layer::custom("replace", json!({"items": ["private-new-value"]})).unwrap())
        .load()
        .unwrap();
    assert!(!format!("{:?}", loaded.report()).contains("private-new-value"));
    assert!(
        !loaded
            .report()
            .audit_json_pretty()
            .contains("private-new-value")
    );
}

#[test]
fn normalizers_keep_historical_secrets_when_container_shapes_change() {
    let loaded = ConfigLoader::new(json!({"items": {"old": "private-historical-value"}}))
        .secret_path("items.old")
        .normalizer("replace", |value| {
            value["items"] = json!(["public"]);
            Ok::<_, String>(())
        })
        .normalizer("update", |value| {
            value["items"][0] = json!("updated");
            Ok::<_, String>(())
        })
        .load()
        .unwrap();
    assert!(!format!("{:?}", loaded.report()).contains("private-historical-value"));
    assert!(
        !loaded
            .report()
            .audit_json_pretty()
            .contains("private-historical-value")
    );
}

#[test]
fn future_secret_array_paths_remain_valid_when_new_array_is_empty() {
    ConfigLoader::new(json!({"items": {"old": "public"}}))
        .secret_path("items[0].token")
        .layer(Layer::custom("replace", json!({"items": []})).unwrap())
        .load()
        .unwrap();
}

#[test]
fn preparation_errors_redact_secret_aliases_when_another_secret_changes_shape() {
    let error = ConfigLoader::new(json!({"token": {}, "items": {"old": "public"}}))
        .secret_path("old_token")
        .secret_path("items[0]")
        .metadata(tier::ConfigMetadata::from_fields([FieldMetadata::new(
            "token",
        )
        .alias("old_token")
        .env_decoder(EnvDecoder::KeyValueMap)]))
        .layer(Layer::custom("replace", json!({"items": []})).unwrap())
        .env(EnvSource::from_pairs([("TOKEN", "private-token")]))
        .load()
        .unwrap_err();
    assert!(!error.to_string().contains("private-token"), "{error}");
}
