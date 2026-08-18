use regex::Regex;
use serde_json::Value;

use super::matches::string::string_matches_schema;

const MAX_GENERATED_STRING_LENGTH: u64 = 1024;

pub(super) fn fallback_string_example(
    object: &serde_json::Map<String, Value>,
    accepts: impl Fn(&str) -> bool,
) -> Option<String> {
    let min_length = object.get("minLength").and_then(Value::as_u64).unwrap_or(0);
    let max_length = object.get("maxLength").and_then(Value::as_u64);
    if max_length.is_some_and(|max_length| min_length > max_length)
        || min_length > MAX_GENERATED_STRING_LENGTH
    {
        return None;
    }

    string_candidate_seeds(object).into_iter().find_map(|seed| {
        let candidate = fit_string_candidate(seed, min_length, max_length)?;
        (string_matches_schema(&candidate, object) && accepts(&candidate)).then_some(candidate)
    })
}

fn string_candidate_seeds(object: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut seeds = format_candidate_seeds(object);
    seeds.extend([
        "example".to_owned(),
        "x".to_owned(),
        String::new(),
        "<secret>".to_owned(),
    ]);
    if let Some(pattern) = object.get("pattern").and_then(Value::as_str)
        && let Some(prefix) = literal_regex_prefix(pattern)
    {
        seeds.insert(0, prefix);
    }

    seeds
}

fn format_candidate_seeds(object: &serde_json::Map<String, Value>) -> Vec<String> {
    match object.get("format").and_then(Value::as_str) {
        Some("uri" | "url" | "uri-reference") => Vec::from(["https://example.com".to_owned()]),
        Some("email" | "idn-email") => Vec::from(["ops@example.com".to_owned()]),
        Some("hostname" | "idn-hostname") => Vec::from(["example.com".to_owned()]),
        Some("ipv4") => Vec::from(["192.0.2.1".to_owned()]),
        Some("ipv6") => Vec::from(["2001:db8::1".to_owned()]),
        _ => Vec::new(),
    }
}

fn fit_string_candidate(
    mut candidate: String,
    min_length: u64,
    max_length: Option<u64>,
) -> Option<String> {
    let min_length = usize::try_from(min_length).ok()?;
    let max_length = max_length.map(usize::try_from).transpose().ok()?;

    let current_length = candidate.chars().count();
    if current_length < min_length {
        candidate.extend(std::iter::repeat_n('x', min_length - current_length));
    }

    if let Some(max_length) = max_length
        && candidate.chars().count() > max_length
    {
        candidate = candidate.chars().take(max_length).collect();
    }

    Some(candidate)
}

fn literal_regex_prefix(pattern: &str) -> Option<String> {
    let pattern = pattern.strip_prefix('^')?;
    let mut prefix = String::new();
    let mut escaped = false;

    for ch in pattern.chars() {
        if escaped {
            prefix.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            ch if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/') => {
                prefix.push(ch);
            }
            _ => break,
        }
    }

    (!prefix.is_empty() && Regex::new(pattern).is_ok_and(|regex| regex.is_match(&prefix)))
        .then_some(prefix)
}
