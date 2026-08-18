use std::ffi::OsStr;

use serde_json::{Map, Value};

use crate::EnvDecoder;

use super::delimited::{parse_csv_fields, parse_key_value_fields};
use super::string_paths::collect_string_leaf_suffixes;
use super::{ParsedOverride, parse_override_value};
use crate::loader::CustomEnvDecoder;

pub(in crate::loader) fn parse_env_override_value(
    raw: &str,
    decoder: Option<EnvDecoder>,
    custom_decoder: Option<&CustomEnvDecoder>,
) -> Result<ParsedOverride, String> {
    match (custom_decoder, decoder) {
        (Some(custom_decoder), _) => {
            let value = custom_decoder(raw)?;
            Ok(ParsedOverride {
                string_coercion_suffixes: collect_string_leaf_suffixes(&value, ""),
                value,
            })
        }
        (None, Some(decoder)) => {
            let value = decode_env_override_value(raw, decoder)?;
            Ok(ParsedOverride {
                string_coercion_suffixes: collect_string_leaf_suffixes(&value, ""),
                value,
            })
        }
        (None, None) => parse_override_value(raw),
    }
}

fn decode_env_override_value(raw: &str, decoder: EnvDecoder) -> Result<Value, String> {
    match decoder {
        EnvDecoder::Csv => Ok(Value::Array(
            parse_csv_fields(raw)?
                .into_iter()
                .map(Value::String)
                .collect(),
        )),
        EnvDecoder::Whitespace => Ok(Value::Array(
            raw.split_whitespace()
                .map(|segment| Value::String(segment.to_owned()))
                .collect(),
        )),
        EnvDecoder::PathList => {
            let values = std::env::split_paths(OsStr::new(raw))
                .map(|path| Value::String(path.to_string_lossy().into_owned()))
                .collect();
            Ok(Value::Array(values))
        }
        EnvDecoder::KeyValueMap => {
            let mut map = Map::new();
            for entry in parse_key_value_fields(raw)? {
                let (key, value) = entry.split_once('=').ok_or_else(|| {
                    format!("invalid key_value_map entry `{entry}`, expected key=value")
                })?;
                let key = key.trim();
                let value = value.trim();
                if key.is_empty() {
                    return Err("key_value_map entries must not use an empty key".to_owned());
                }
                map.insert(key.to_owned(), Value::String(value.to_owned()));
            }
            Ok(Value::Object(map))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::decode_env_override_value;
    use crate::EnvDecoder;

    #[test]
    fn csv_decoder_supports_quoted_commas_and_escaped_quotes() {
        let decoded = decode_env_override_value(
            r##"localhost,"api,internal","quote ""ok""""##,
            EnvDecoder::Csv,
        )
        .expect("csv decodes");

        assert_eq!(
            decoded,
            json!(["localhost", "api,internal", "quote \"ok\""])
        );
    }

    #[test]
    fn csv_decoder_ignores_unquoted_empty_segments_but_keeps_quoted_empty_values() {
        let decoded = decode_env_override_value(r#"alpha,, "", beta,"#, EnvDecoder::Csv)
            .expect("csv decodes");

        assert_eq!(decoded, json!(["alpha", "", "beta"]));
    }

    #[test]
    fn csv_decoder_rejects_unterminated_quotes() {
        let error = decode_env_override_value(r#""alpha,beta"#, EnvDecoder::Csv)
            .expect_err("unterminated quote should fail");

        assert!(error.contains("unterminated quoted csv field"));
    }

    #[test]
    fn key_value_map_decoder_supports_quoted_entries() {
        let decoded = decode_env_override_value(
            r#"http=80,"description=api, internal","quote=a ""quoted"" value""#,
            EnvDecoder::KeyValueMap,
        )
        .expect("key value map decodes");

        assert_eq!(
            decoded,
            json!({
                "http": "80",
                "description": "api, internal",
                "quote": "a \"quoted\" value"
            })
        );
    }

    #[test]
    fn key_value_map_decoder_supports_quoted_values() {
        let decoded = decode_env_override_value(
            r#"http=80,description="api, internal",quote="a ""quoted"" value""#,
            EnvDecoder::KeyValueMap,
        )
        .expect("key value map decodes quoted values");

        assert_eq!(
            decoded,
            json!({
                "http": "80",
                "description": "api, internal",
                "quote": "a \"quoted\" value"
            })
        );
    }

    #[test]
    fn key_value_map_decoder_rejects_unterminated_quoted_values() {
        let error =
            decode_env_override_value(r#"description="api, internal"#, EnvDecoder::KeyValueMap)
                .expect_err("unterminated key value quote should fail");

        assert!(error.contains("unterminated quoted key_value_map field"));
    }
}
