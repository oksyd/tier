#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde::{Deserialize, Serialize};
use tier::{ConfigError, ConfigLoader, EnvSource};

#[derive(Debug, Serialize, Deserialize)]
struct Count {
    count: u32,
}
#[derive(Debug, Serialize, Deserialize)]
struct Flat {
    name: String,
    #[serde(flatten)]
    nested: Count,
}

#[test]
fn flatten_retry_preserves_successfully_deserialized_string_fields() {
    for name in ["123", "true", "null", "1.25"] {
        let loaded = ConfigLoader::new(Flat {
            name: "default".into(),
            nested: Count { count: 0 },
        })
        .env(EnvSource::from_pairs([("APP_NAME", name), ("APP_COUNT", "7")]).prefix("APP"))
        .load()
        .unwrap();
        assert_eq!(loaded.config().name, name);
        assert_eq!(loaded.config().nested.count, 7);
        assert_eq!(loaded.report().redacted_final_value()["name"], name);
        assert_eq!(loaded.report().redacted_final_value()["count"], 7);
    }
}

#[test]
fn flatten_retry_preserves_nested_strings_and_numeric_enum_names() {
    #[derive(Debug, Serialize, Deserialize)]
    enum Mode {
        #[serde(rename = "123")]
        Numeric,
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct Names {
        label: String,
        mode: Mode,
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct App {
        names: Names,
        #[serde(flatten)]
        count: Count,
    }
    let loaded = ConfigLoader::new(App {
        names: Names {
            label: "default".into(),
            mode: Mode::Numeric,
        },
        count: Count { count: 0 },
    })
    .env(
        EnvSource::from_pairs([
            ("APP_NAMES__LABEL", "123"),
            ("APP_NAMES__MODE", "123"),
            ("APP_COUNT", "7"),
        ])
        .prefix("APP"),
    )
    .load()
    .unwrap();
    assert_eq!(loaded.config().names.label, "123");
    assert!(matches!(loaded.config().names.mode, Mode::Numeric));
    assert_eq!(loaded.config().count.count, 7);
}

#[test]
fn flatten_retry_does_not_hide_unknown_fields() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Settings {
        name: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct App {
        settings: Settings,
        #[serde(flatten)]
        nested: Count,
    }
    let error = ConfigLoader::new(App {
        settings: Settings {
            name: "default".into(),
        },
        nested: Count { count: 0 },
    })
    .env(
        EnvSource::from_pairs([
            ("APP_SETTINGS__NAME", "123"),
            ("APP_COUNT", "7"),
            ("APP_SETTINGS__TYPO", "extra"),
        ])
        .prefix("APP"),
    )
    .load()
    .unwrap_err();
    assert!(
        matches!(error, ConfigError::UnknownFields { fields } if fields.iter().any(|field| field.path == "settings.typo"))
    );
}
