use std::collections::BTreeSet;

use serde_json::Value;

use super::ParsedOverride;

pub(in crate::loader) fn parse_override_value(raw: &str) -> Result<ParsedOverride, String> {
    if raw.is_empty() {
        return Ok(ParsedOverride {
            value: Value::String(String::new()),
            string_coercion_suffixes: BTreeSet::from([String::new()]),
        });
    }

    let trimmed = raw.trim();
    let uses_explicit_json_syntax =
        matches!(trimmed.chars().next(), Some('{') | Some('[') | Some('"'));

    if uses_explicit_json_syntax {
        let value = serde_json::from_str::<Value>(trimmed)
            .map_err(|error| format!("invalid explicit JSON override: {error}"))?;
        return Ok(ParsedOverride {
            value,
            string_coercion_suffixes: BTreeSet::new(),
        });
    }

    Ok(ParsedOverride {
        value: Value::String(raw.to_owned()),
        string_coercion_suffixes: BTreeSet::from([String::new()]),
    })
}
