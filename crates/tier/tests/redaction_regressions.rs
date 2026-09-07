#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::json;
use tier::{ConfigError, ConfigLoader, ConfigMetadata};

#[test]
fn required_if_redacts_secret_trigger_messages_and_payloads() {
    for secret in ["users", "users.*.token", "users[0].token"] {
        let error = ConfigLoader::new(json!({"users": [{"token": "private-credential"}]}))
            .secret_path(secret)
            .metadata(ConfigMetadata::new().required_if(
                "users.*.token",
                "private-credential",
                ["users.*.cert"],
            ))
            .load()
            .unwrap_err();
        assert!(
            !error.to_string().contains("private-credential"),
            "{secret}: {error}"
        );
        assert!(!format!("{error:?}").contains("private-credential"));
        let ConfigError::Validation { failures } = error else {
            panic!("expected a validation failure");
        };
        let failure = failures
            .iter()
            .next()
            .unwrap()
            .errors
            .iter()
            .next()
            .unwrap();
        assert!(
            failure
                .message
                .contains("users.0.token == ***redacted*** requires users.0.cert")
        );
        assert_eq!(failure.expected, Some(json!("***redacted***")));
        assert!(
            !serde_json::to_string(&failures)
                .unwrap()
                .contains("private-credential")
        );
    }
}

#[test]
fn required_if_redacts_object_comparisons_containing_secret_fields() {
    let credential = json!({"token": "private-credential"});
    let error = ConfigLoader::new(json!({"credentials": credential}))
        .secret_path("credentials.token")
        .metadata(ConfigMetadata::new().required_if(
            "credentials",
            tier::ValidationValue(credential),
            ["cert"],
        ))
        .load()
        .unwrap_err();
    assert!(!error.to_string().contains("private-credential"));
    assert!(!format!("{error:?}").contains("private-credential"));
}

#[test]
fn required_if_preserves_public_trigger_messages() {
    let error = ConfigLoader::new(json!({"mode": "production"}))
        .secret_path("cert")
        .metadata(ConfigMetadata::new().required_if("mode", "production", ["cert"]))
        .load()
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("mode == \"production\" requires cert")
    );
}

#[cfg(feature = "schema")]
mod schema {
    use std::borrow::Cow;

    use schemars::JsonSchema;
    use serde::Deserialize;
    use tier::{FieldMetadata, TierMetadata, annotated_json_schema_for};

    use super::*;

    fn secret_default() -> String {
        "private-default".to_owned()
    }

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct DerivedAnnotations {
        #[serde(default = "secret_default")]
        #[schemars(example = "private-example")]
        token: String,
        #[serde(default)]
        #[schemars(example = "public-example")]
        public: String,
    }

    impl TierMetadata for DerivedAnnotations {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([FieldMetadata::new("token").secret()])
        }
    }

    #[test]
    fn schema_redacts_schemars_defaults_and_plural_examples() {
        let schema = annotated_json_schema_for::<DerivedAnnotations>();
        assert_eq!(schema["properties"]["token"]["default"], "<secret>");
        assert_eq!(
            schema["properties"]["token"]["examples"],
            json!(["<secret>"])
        );
        assert_eq!(schema["properties"]["public"]["default"], "");
        assert_eq!(
            schema["properties"]["public"]["examples"],
            json!(["public-example"])
        );
        assert!(!schema.to_string().contains("private-"));
    }

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct NestedDerivedAnnotations {
        inner: DerivedAnnotations,
    }

    impl TierMetadata for NestedDerivedAnnotations {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([FieldMetadata::new("inner.token").secret()])
        }
    }

    #[test]
    fn schema_drops_unused_definitions_after_secret_metadata_inlines_their_reference() {
        let schema = annotated_json_schema_for::<NestedDerivedAnnotations>();
        assert_eq!(
            schema["properties"]["inner"]["properties"]["token"]["default"],
            "<secret>"
        );
        assert!(!schema.to_string().contains("private-"), "{schema}");
        assert!(schema.to_string().contains("public-example"));
    }

    struct NestedAnnotations;

    impl TierMetadata for NestedAnnotations {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([
                FieldMetadata::new("credentials").secret(),
                FieldMetadata::new("users.*.token").secret(),
            ])
        }
    }

    impl JsonSchema for NestedAnnotations {
        fn schema_name() -> Cow<'static, str> {
            Cow::Borrowed("NestedAnnotations")
        }

        fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            serde_json::from_value(json!({
                "type": "object",
                "default": {
                    "credentials": {"token": "private-parent-default"},
                    "users": [{"token": "private-user-default", "name": "public-name"}]
                },
                "examples": [{
                    "credentials": {"token": "private-parent-example"},
                    "users": [{"token": "private-user-example", "name": "public-name"}]
                }],
                "properties": {
                    "credentials": {
                        "type": "object",
                        "properties": {
                            "token": {
                                "type": "string",
                                "default": "private-nested-default",
                                "example": "private-nested-example",
                                "examples": ["private-nested-examples"]
                            }
                        }
                    },
                    "users": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "token": {"type": "string"},
                                "name": {"type": "string", "default": "public-name"}
                            }
                        }
                    },
                    "public": {
                        "default": {"writeOnly": true, "example": "public-data"}
                    }
                }
            }))
            .unwrap()
        }
    }

    #[test]
    fn schema_redacts_secret_ancestors_and_nested_values_in_parent_annotations() {
        let schema = annotated_json_schema_for::<NestedAnnotations>();
        assert!(!schema.to_string().contains("private-"), "{schema}");
        assert_eq!(schema["default"]["users"][0]["token"], "<secret>");
        assert_eq!(schema["default"]["users"][0]["name"], "public-name");
        assert_eq!(schema["examples"][0]["credentials"]["token"], "<secret>");
        assert_eq!(
            schema["properties"]["credentials"]["properties"]["token"]["examples"],
            json!(["<secret>"])
        );
        assert_eq!(
            schema["properties"]["public"]["default"]["example"],
            "public-data"
        );
    }

    struct RefAnnotations;

    impl TierMetadata for RefAnnotations {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([FieldMetadata::new("credentials").secret()])
        }
    }

    impl JsonSchema for RefAnnotations {
        fn schema_name() -> Cow<'static, str> {
            Cow::Borrowed("RefAnnotations")
        }

        fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
            serde_json::from_value(json!({
                "type": "object",
                "properties": {
                    "credentials": {"$ref": "#/$defs/Credentials"},
                    "intrinsic": {"$ref": "#/$defs/Secret"},
                    "public": {"$ref": "#/$defs/Public"}
                },
                "$defs": {
                    "Credentials": {
                        "type": "object",
                        "default": {"token": "private-ref-default"},
                        "examples": [{"token": "private-ref-example"}],
                        "properties": {
                            "token": {"$ref": "#/$defs/Token"},
                            "nested": {"$ref": "#/$defs/Credentials"}
                        }
                    },
                    "Token": {"type": "string", "default": "private-token-default"},
                    "Secret": {
                        "type": "string",
                        "writeOnly": true,
                        "default": "private-intrinsic-default",
                        "examples": ["private-intrinsic-example"]
                    },
                    "Public": {"type": "string", "default": "public-default", "examples": ["public-example"]}
                }
            })).unwrap()
        }
    }

    #[test]
    fn schema_redacts_referenced_annotations_and_recursive_secret_definitions() {
        let schema = annotated_json_schema_for::<RefAnnotations>();
        assert!(!schema.to_string().contains("private-"), "{schema}");
        assert_eq!(
            schema["properties"]["credentials"]["default"]["token"],
            "<secret>"
        );
        assert_eq!(
            schema["properties"]["intrinsic"]["examples"],
            json!(["<secret>"])
        );
        assert_eq!(schema["$defs"]["Token"]["default"], "<secret>");
        assert_eq!(schema["$defs"]["Public"]["default"], "public-default");
    }
}
