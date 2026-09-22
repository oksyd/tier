#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(feature = "derive")]
mod derive {
    use serde::{Deserialize, Serialize};
    use tier::{ConfigLoader, EnvSource, TierConfig, TierMetadata};

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind", rename_all_fields = "SCREAMING_SNAKE_CASE")]
    enum VariantNames {
        #[serde(rename_all = "camelCase")]
        Http {
            #[tier(env = "AUTH_TOKEN", secret)]
            auth_token: String,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
    struct FlattenNames {
        #[serde(flatten)]
        common_fields: CommonFields,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    struct CommonFields {
        host_name: String,
    }

    #[test]
    fn directional_container_renames_do_not_rename_flattened_fields() {
        let loaded = ConfigLoader::new(FlattenNames {
            common_fields: CommonFields {
                host_name: "default".to_owned(),
            },
        })
        .derive_metadata()
        .env(EnvSource::from_pairs([("HOST_NAME", "updated")]))
        .load()
        .unwrap();
        assert_eq!(loaded.common_fields.host_name, "updated");
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind")]
    enum SharedSecret {
        A {
            #[tier(secret)]
            token: String,
        },
        B {
            token: String,
        },
    }

    #[test]
    fn enum_field_conflicts_preserve_secret_metadata() {
        let loaded = ConfigLoader::new(SharedSecret::A {
            token: "private-token".to_owned(),
        })
        .derive_metadata()
        .load()
        .unwrap();
        assert!(
            !loaded
                .report()
                .redacted_pretty_json()
                .contains("private-token")
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(rename_all_fields = "SCREAMING_SNAKE_CASE")]
    enum DirectionalVariantNames {
        #[serde(rename_all(serialize = "camelCase", deserialize = "kebab-case"))]
        Http { auth_token: String },
    }

    #[test]
    fn directional_variant_renames_override_container_field_rules() {
        let metadata = DirectionalVariantNames::metadata();
        let field = metadata.field("Http.auth-token").unwrap();
        assert!(field.aliases().contains(&"Http.authToken".to_owned()));
        ConfigLoader::new(DirectionalVariantNames::Http {
            auth_token: "default".to_owned(),
        })
        .derive_metadata()
        .load()
        .unwrap();
    }

    #[derive(tier::TierPatch)]
    #[serde(rename_all_fields = "SCREAMING_SNAKE_CASE")]
    enum RenamedPatch {
        #[serde(rename_all(deserialize = "camelCase"))]
        Http { auth_token: Option<String> },
    }

    #[test]
    fn typed_patch_variant_renames_match_deserialization_paths() {
        let patch = RenamedPatch::Http {
            auth_token: Some("patched".to_owned()),
        };
        let loaded = ConfigLoader::new(serde_json::json!({ "authToken": "default" }))
            .patch("patch", &patch)
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(
            loaded.config(),
            &serde_json::json!({ "authToken": "patched" })
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    struct NestedSecret {
        #[tier(secret)]
        token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(untagged)]
    enum SharedNestedSecret {
        A { credentials: NestedSecret },
        B { credentials: String },
    }

    #[test]
    fn enum_field_conflicts_preserve_nested_secret_boundaries() {
        let loaded = ConfigLoader::new(SharedNestedSecret::A {
            credentials: NestedSecret {
                token: "private-token".to_owned(),
            },
        })
        .derive_metadata()
        .load()
        .unwrap();
        assert!(
            !loaded
                .report()
                .redacted_pretty_json()
                .contains("private-token")
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind")]
    enum SharedPolicy {
        A {
            #[tier(sources("default", "cli"), deny_sources("env"))]
            token: String,
        },
        B {
            #[tier(sources("default", "file"), deny_sources("custom"))]
            token: String,
        },
    }

    #[test]
    fn shared_enum_source_policies_intersect_allowlists_and_union_denylists() {
        use tier::SourceKind;
        let metadata = SharedPolicy::metadata();
        let field = metadata.field("token").unwrap();
        assert_eq!(
            field.allowed_sources().unwrap(),
            &std::collections::BTreeSet::from([SourceKind::Default])
        );
        assert_eq!(
            field.denied_sources().unwrap(),
            &std::collections::BTreeSet::from([SourceKind::Environment, SourceKind::Custom])
        );
        let error = ConfigLoader::new(SharedPolicy::A {
            token: "default".to_owned(),
        })
        .derive_metadata()
        .env(EnvSource::from_pairs([
            ("KIND", "A"),
            ("TOKEN", "env-write"),
        ]))
        .load()
        .unwrap_err();
        assert!(
            matches!(error, tier::ConfigError::SourcePolicyViolation { path, .. } if path == "token")
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    struct RestrictedNested {
        #[tier(deny_sources("env"))]
        token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind", content = "config")]
    enum SharedNestedPolicy {
        A {
            credentials: RestrictedNested,
        },
        B {
            credentials: CredentialsWithoutPolicy,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    struct CredentialsWithoutPolicy {
        token: String,
    }

    #[test]
    fn shared_enum_fields_keep_nested_source_policies() {
        let metadata = SharedNestedPolicy::metadata();
        assert!(
            metadata
                .field("config.credentials.token")
                .unwrap()
                .denied_sources()
                .unwrap()
                .contains(&tier::SourceKind::Environment)
        );
        let error = ConfigLoader::new(SharedNestedPolicy::A {
            credentials: RestrictedNested {
                token: "default".to_owned(),
            },
        })
        .derive_metadata()
        .env(EnvSource::from_pairs([
            ("KIND", "A"),
            ("CONFIG", r#"{"credentials":{"token":"env-write"}}"#),
        ]))
        .load()
        .unwrap_err();
        assert!(
            matches!(error, tier::ConfigError::SourcePolicyViolation { path, .. } if path == "config.credentials.token")
        );
    }

    #[test]
    fn variant_rename_all_controls_metadata_and_environment_writes() {
        let metadata = VariantNames::metadata();
        assert!(metadata.field("authToken").is_some());
        let loaded = ConfigLoader::new(VariantNames::Http {
            auth_token: "default".to_owned(),
        })
        .derive_metadata()
        .env(EnvSource::from_pairs([
            ("AUTH_TOKEN", "private-token"),
            ("KIND", "Http"),
        ]))
        .load()
        .unwrap();
        let VariantNames::Http { auth_token } = loaded.config();
        assert_eq!(auth_token, "private-token");
        assert!(
            !loaded
                .report()
                .redacted_pretty_json()
                .contains("private-token")
        );
    }
}

#[cfg(feature = "schema")]
mod schema {
    use schemars::JsonSchema;
    use serde::Deserialize;
    use tier::{
        ConfigMetadata, EnvDocOptions, FieldMetadata, TierMetadata, config_example_for,
        env_docs_for,
    };

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Credentials {
        token: String,
    }

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct SecretParent {
        credentials: Credentials,
    }

    impl TierMetadata for SecretParent {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([
                FieldMetadata::new("credentials").secret(),
                FieldMetadata::new("credentials.token").example("private-token"),
            ])
        }
    }

    #[test]
    fn env_docs_redact_metadata_examples_below_secret_ancestors() {
        let docs = env_docs_for::<SecretParent>(&EnvDocOptions::new());
        assert!(docs[0].secret);
        assert_eq!(docs[0].example.as_deref(), Some("<secret>"));
    }

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct SchemaSecretParent {
        #[serde(skip_serializing)]
        credentials: Credentials,
    }

    impl TierMetadata for SchemaSecretParent {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([
                FieldMetadata::new("credentials.token").example("private-token")
            ])
        }
    }

    #[test]
    fn env_docs_inherit_schema_secret_markers_across_references() {
        let docs = env_docs_for::<SchemaSecretParent>(&EnvDocOptions::new());
        assert!(docs[0].secret);
        assert_eq!(docs[0].example.as_deref(), Some("<secret>"));
    }

    struct FilteredExamples;

    impl JsonSchema for FilteredExamples {
        fn schema_name() -> std::borrow::Cow<'static, str> {
            "FilteredExamples".into()
        }
        fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "port": { "type": "integer", "minimum": 1024, "examples": ["wrong-type", 80, 8080], "default": 9000 },
                    "host": { "type": "string", "examples": ["schema-host"] }
                }
            }).try_into().unwrap()
        }
    }

    impl TierMetadata for FilteredExamples {
        fn metadata() -> ConfigMetadata {
            ConfigMetadata::from_fields([FieldMetadata::new("host").example("metadata-host")])
        }
    }

    #[test]
    fn standard_examples_skip_invalid_values_and_respect_metadata_priority() {
        let example = config_example_for::<FilteredExamples>();
        assert_eq!(example["port"], 8080);
        assert_eq!(example["host"], "metadata-host");
    }

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct SchemaExamples {
        #[schemars(example = "production-host")]
        host: String,
    }

    impl TierMetadata for SchemaExamples {}

    #[test]
    fn config_examples_honor_standard_schema_examples() {
        assert_eq!(
            config_example_for::<SchemaExamples>()["host"],
            "production-host"
        );
    }
}

#[cfg(feature = "derive")]
mod conflicting_aliases {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tier::{ConfigError, ConfigLoader, Layer, TierConfig};

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind")]
    enum SecretAlias {
        A {
            #[serde(alias = "legacy")]
            #[tier(secret)]
            token: String,
        },
        B {
            #[serde(alias = "legacy")]
            value: String,
        },
    }

    #[test]
    fn ambiguous_enum_aliases_keep_secret_markers_without_rewriting() {
        let loaded = ConfigLoader::new(SecretAlias::B {
            value: "public".to_owned(),
        })
        .derive_metadata()
        .layer(Layer::custom("switch", json!({"kind": "A", "legacy": "private-token"})).unwrap())
        .load()
        .unwrap();
        assert!(matches!(loaded.config(), SecretAlias::A { token } if token == "private-token"));
        assert!(
            !loaded
                .report()
                .audit_json_pretty()
                .contains("private-token")
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind")]
    enum RestrictedAlias {
        A {
            #[serde(alias = "legacy")]
            #[tier(deny_sources("custom"))]
            token: String,
        },
        B {
            #[serde(alias = "legacy")]
            value: String,
        },
    }

    #[test]
    fn ambiguous_enum_aliases_keep_source_restrictions() {
        let error = ConfigLoader::new(RestrictedAlias::B {
            value: "public".to_owned(),
        })
        .derive_metadata()
        .layer(Layer::custom("switch", json!({"kind": "A", "legacy": "private-token"})).unwrap())
        .load()
        .unwrap_err();
        assert!(
            matches!(error, ConfigError::SourcePolicyViolation { path, .. } if path == "legacy")
        );
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    struct Credentials {
        #[tier(secret)]
        token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, TierConfig)]
    #[serde(tag = "kind")]
    enum NestedAlias {
        A {
            #[serde(alias = "legacy")]
            credentials: Credentials,
        },
        B {
            #[serde(alias = "legacy")]
            value: String,
        },
    }

    #[test]
    fn ambiguous_enum_aliases_keep_nested_secret_metadata() {
        let loaded = ConfigLoader::new(NestedAlias::B {
            value: "public".to_owned(),
        })
        .derive_metadata()
        .layer(
            Layer::custom(
                "switch",
                json!({"kind": "A", "legacy": {"token": "private-token"}}),
            )
            .unwrap(),
        )
        .load()
        .unwrap();
        assert!(
            !loaded
                .report()
                .audit_json_pretty()
                .contains("private-token")
        );
    }
}
