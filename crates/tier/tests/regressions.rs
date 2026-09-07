#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tier::{
    ArgsSource, ConfigError, ConfigLoader, ConfigMetadata, EnvDecoder, EnvSource, FieldMetadata,
    Layer, MergeStrategy, ReloadEvent, ReloadHandle,
};

#[derive(Debug, Serialize, Deserialize)]
struct NumberConfig {
    value: u32,
}

#[test]
fn sensitive_deserialization_errors_and_reload_events_do_not_expose_input() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handle = ReloadHandle::new(move || {
        let value = if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            "7"
        } else {
            "private-credential"
        };
        ConfigLoader::new(NumberConfig { value: 0 })
            .secret_path("value")
            .env(EnvSource::from_pairs([("APP_VALUE", value)]).prefix("APP"))
            .load()
    })
    .unwrap();
    let events = handle.subscribe();
    let error = handle.reload().unwrap_err();
    assert!(
        matches!(&error, ConfigError::Deserialize { path, provenance: Some(_), .. } if path == "value")
    );
    assert!(!error.to_string().contains("private-credential"));
    assert!(!format!("{error:?}").contains("private-credential"));
    assert!(error.to_string().contains("redacted"));
    assert!(!handle.last_error().unwrap().contains("private-credential"));
    let event = events.try_recv().unwrap();
    assert!(matches!(event, ReloadEvent::Rejected(_)));
    assert!(!format!("{event:?}").contains("private-credential"));
    assert_eq!(handle.snapshot().config().value, 7);
}

#[test]
fn ordinary_deserialization_errors_keep_their_details() {
    let error = ConfigLoader::new(NumberConfig { value: 0 })
        .env(EnvSource::from_pairs([("APP_VALUE", "not-a-number")]).prefix("APP"))
        .load()
        .unwrap_err();
    assert!(error.to_string().contains("not-a-number"));
    assert!(error.to_string().contains("expected u32"));
}

#[test]
fn secret_ancestor_and_wildcard_paths_redact_deserialization_errors() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Nested {
        items: Vec<NumberConfig>,
    }
    for path in ["items", "items.*.value", "items[0].value"] {
        let error = ConfigLoader::new(Nested {
            items: vec![NumberConfig { value: 1 }],
        })
        .secret_path(path)
        .args(ArgsSource::from_args([
            "app",
            "--set",
            "items[0].value=private-credential",
        ]))
        .load()
        .unwrap_err();
        assert!(
            !error.to_string().contains("private-credential"),
            "{path}: {error}"
        );
    }
}

fn append_metadata(path: &str) -> ConfigMetadata {
    ConfigMetadata::from_fields([FieldMetadata::new(path).merge_strategy(MergeStrategy::Append)])
}

#[test]
fn appended_secrets_are_redacted_in_every_trace_at_the_destination_index() {
    let loaded = ConfigLoader::new(json!({"values": ["public"]}))
        .metadata(append_metadata("values"))
        .secret_path("values[1]")
        .layer(Layer::custom("first-append", json!({"values": ["private-credential"]})).unwrap())
        .layer(Layer::custom("second-append", json!({"values": ["another-public"]})).unwrap())
        .load()
        .unwrap();
    let report = loaded.report();
    assert_eq!(
        report.redacted_final_value(),
        &json!({"values": ["public", "***redacted***", "another-public"]})
    );
    assert!(!format!("{report:?}").contains("private-credential"));
    assert!(
        !report
            .audit_json()
            .to_string()
            .contains("private-credential")
    );
    assert_eq!(report.traces()["values.0"].len(), 1);
    assert_eq!(report.traces()["values.1"][0].source.name, "first-append");
    assert_eq!(report.traces()["values.1"][0].value, "***redacted***");
    assert_eq!(report.traces()["values.2"][0].source.name, "second-append");
}

#[test]
fn appends_within_indexed_patches_align_nested_secret_traces() {
    let loaded = ConfigLoader::new(json!({"groups": [{"values": ["public"]}]}))
        .metadata(append_metadata("groups.*.values"))
        .secret_path("groups[0].values[1]")
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"groups[0].values=["private-credential"]"#,
        ]))
        .load()
        .unwrap();
    assert!(!format!("{:?}", loaded.report()).contains("private-credential"));
    assert_eq!(
        loaded.report().traces()["groups.0.values"]
            .last()
            .unwrap()
            .value,
        json!([null, "***redacted***"])
    );
}

#[derive(Debug, Serialize, Deserialize)]
struct StringConfig {
    value: Option<String>,
}

#[test]
fn explicit_json_overrides_do_not_inherit_env_coercion() {
    let loaded = ConfigLoader::new(StringConfig { value: None })
        .env(EnvSource::from_pairs([("APP_VALUE", "abc")]).prefix("APP"))
        .args(ArgsSource::from_args(["app", "--set", r#"value="null""#]))
        .load()
        .unwrap();
    assert_eq!(loaded.config().value.as_deref(), Some("null"));
}

#[test]
fn replacing_parent_containers_clears_descendant_coercion() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Nested {
        nested: StringConfig,
    }
    let loaded = ConfigLoader::new(Nested {
        nested: StringConfig { value: None },
    })
    .env(EnvSource::from_pairs([("APP_NESTED__VALUE", "abc")]).prefix("APP"))
    .args(ArgsSource::from_args([
        "app",
        "--set",
        r#"nested={"value":"null"}"#,
    ]))
    .load()
    .unwrap();
    assert_eq!(loaded.config().nested.value.as_deref(), Some("null"));
}

#[test]
fn replacing_array_in_same_arg_layer_clears_previous_element_coercion() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Strings {
        values: Vec<Option<String>>,
    }
    let loaded = ConfigLoader::new(Strings { values: vec![None] })
        .args(ArgsSource::from_args([
            "app",
            "--set",
            "values[0]=abc",
            "--set",
            r#"values=["null"]"#,
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.config().values, vec![Some("null".to_owned())]);
}

#[derive(Debug, Serialize, Deserialize)]
struct Numbers {
    values: Vec<Option<u32>>,
}

#[test]
fn consecutive_csv_appends_coerce_all_destination_elements() {
    let loaded = ConfigLoader::new(Numbers {
        values: vec![Some(7)],
    })
    .metadata(append_metadata("values"))
    .env_decoder("values", EnvDecoder::Csv)
    .env(EnvSource::from_pairs([("APP_VALUES", "8,9")]).prefix("APP"))
    .env(EnvSource::from_pairs([("APP_VALUES", "10,11")]).prefix("APP"))
    .load()
    .unwrap();
    assert_eq!(
        loaded.config().values,
        vec![Some(7), Some(8), Some(9), Some(10), Some(11)]
    );
}

#[test]
fn replacing_an_array_clears_coercion_from_earlier_env_arrays() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Strings {
        values: Vec<Option<String>>,
    }
    let loaded = ConfigLoader::new(Strings { values: vec![] })
        .env_decoder("values", EnvDecoder::Csv)
        .env(EnvSource::from_pairs([("APP_VALUES", "abc,def")]).prefix("APP"))
        .args(ArgsSource::from_args([
            "app",
            "--set",
            r#"values=["null"]"#,
        ]))
        .load()
        .unwrap();
    assert_eq!(loaded.config().values, vec![Some("null".to_owned())]);
}

#[test]
fn append_shape_contains_defaults_exactly_once() {
    let loaded = ConfigLoader::new(json!({"values": [{"0": "first"}]}))
        .metadata(append_metadata("values"))
        .layer(Layer::custom("append", json!({"values": [[1]]})).unwrap())
        .args(ArgsSource::from_args(["app", "--set", "values[1][0]=2"]))
        .load()
        .unwrap();
    assert_eq!(loaded.config(), &json!({"values": [{"0": "first"}, ["2"]]}));
}

#[test]
fn reload_diff_distinguishes_missing_and_null_fields_and_elements() {
    for (before, after, path) in [
        (json!({}), json!({"new": null}), "new"),
        (json!({"old": null}), json!({}), "old"),
        (json!({"items": []}), json!({"items": [null]}), "items.0"),
        (json!({"items": [null]}), json!({"items": []}), "items.0"),
    ] {
        let calls = AtomicUsize::new(0);
        let before_clone = before.clone();
        let after_clone = after.clone();
        let handle = ReloadHandle::new(move || {
            ConfigLoader::new(if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                before_clone.clone()
            } else {
                after_clone.clone()
            })
            .load()
        })
        .unwrap();
        let summary = handle.reload_detailed().unwrap();
        assert!(summary.had_changes);
        assert!(summary.changed_paths.iter().any(|changed| changed == path));
        let change = summary
            .changes
            .iter()
            .find(|change| change.path == path)
            .unwrap();
        let pointer = format!("/{}", path.replace('.', "/"));
        assert_eq!(change.before, before.pointer(&pointer).cloned());
        assert_eq!(change.after, after.pointer(&pointer).cloned());
    }
}

#[cfg(feature = "derive")]
#[test]
fn typed_null_array_patch_clears_only_explicit_elements_and_can_extend_with_null() {
    #[derive(tier::TierPatch)]
    struct Clear {
        #[tier(path = "values[1]")]
        middle: tier::Patch<Option<u32>>,
        #[tier(path = "values[3]")]
        last: tier::Patch<Option<u32>>,
    }
    let patch = Clear {
        middle: tier::Patch::Set(None),
        last: tier::Patch::Set(None),
    };
    for deferred in [false, true] {
        let loader = ConfigLoader::new(Numbers {
            values: vec![Some(7), Some(8), Some(9)],
        });
        let loader = if deferred {
            loader.patch("clear", &patch).unwrap()
        } else {
            loader.layer(Layer::from_patch("clear", &patch).unwrap())
        };
        let loaded = loader.load().unwrap();
        assert_eq!(loaded.config().values, vec![Some(7), None, Some(9), None]);
        assert_eq!(
            loaded.report().traces()["values.1"].last().unwrap().value,
            json!(null)
        );
    }
}

#[cfg(feature = "derive")]
#[test]
fn documented_flattened_and_skipped_fields_derive_without_metadata_errors() {
    #[derive(Debug, Serialize, Deserialize, tier::TierConfig)]
    struct Inner {
        /// Listening port.
        port: u16,
    }
    #[derive(Debug, Serialize, Deserialize, tier::TierConfig)]
    struct Outer {
        /// Shared options.
        #[serde(flatten)]
        inner: Inner,
        /// Runtime cache.
        #[serde(skip)]
        cache: String,
    }
    let config = Outer {
        inner: Inner { port: 7 },
        cache: String::new(),
    };
    assert!(config.cache.is_empty());
    let metadata = <Outer as tier::TierMetadata>::metadata();
    assert!(metadata.fields().iter().any(|field| field.path() == "port"));
    assert!(
        !metadata
            .fields()
            .iter()
            .any(|field| field.path() == "cache")
    );
    ConfigLoader::new(config).metadata(metadata).load().unwrap();
}

#[test]
fn append_deprecations_use_destination_paths_without_warning_for_padding() {
    let loaded = ConfigLoader::new(json!({"values": ["old"]}))
        .metadata(ConfigMetadata::from_fields([
            FieldMetadata::new("values").merge_strategy(MergeStrategy::Append),
            FieldMetadata::new("values.0").deprecated("untouched-default"),
            FieldMetadata::new("values.1").deprecated("newly-appended"),
        ]))
        .layer(Layer::custom("append", json!({"values": ["new"]})).unwrap())
        .load()
        .unwrap();
    assert_eq!(loaded.report().warnings().len(), 1);
    let warning = loaded.report().warnings()[0].to_string();
    assert!(warning.contains("newly-appended"));
    assert!(!warning.contains("untouched-default"));
}

#[test]
fn csv_appends_inside_indexed_arrays_coerce_destination_paths() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Groups {
        groups: Vec<Numbers>,
    }
    let loaded = ConfigLoader::new(Groups {
        groups: vec![Numbers {
            values: vec![Some(7)],
        }],
    })
    .metadata(append_metadata("groups.*.values"))
    .env_decoder("groups.*.values", EnvDecoder::Csv)
    .env(EnvSource::from_pairs([("APP_GROUPS__0__VALUES", "8,9")]).prefix("APP"))
    .load()
    .unwrap();
    assert_eq!(
        loaded.config().groups[0].values,
        vec![Some(7), Some(8), Some(9)]
    );
}

#[test]
fn parent_deserializer_errors_cannot_disclose_sensitive_descendants() {
    fn reject<'de, D: serde::Deserializer<'de>>(de: D) -> Result<serde_json::Value, D::Error> {
        let value = serde_json::Value::deserialize(de)?;
        Err(serde::de::Error::custom(format!("invalid object: {value}")))
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct Parent {
        #[serde(deserialize_with = "reject")]
        nested: serde_json::Value,
    }
    let error = ConfigLoader::new(Parent {
        nested: json!({"password": "private-credential"}),
    })
    .secret_path("nested.password")
    .load()
    .unwrap_err();
    assert!(matches!(&error, ConfigError::Deserialize { path, .. } if path == "nested"));
    assert!(!error.to_string().contains("private-credential"));
}
