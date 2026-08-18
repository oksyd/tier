use regex::Regex;
use serde_json::Value;

use crate::formats::{is_valid_email, is_valid_hostname, is_valid_url};
use crate::schema::count::{keyword_u64, len_at_least, len_at_most};

pub(in crate::schema::example) fn string_matches_schema(
    text: &str,
    object: &serde_json::Map<String, Value>,
) -> bool {
    let char_count = text.chars().count();
    if keyword_u64(object, "minLength")
        .is_some_and(|min_length| !len_at_least(char_count, min_length))
    {
        return false;
    }
    if keyword_u64(object, "maxLength")
        .is_some_and(|max_length| !len_at_most(char_count, max_length))
    {
        return false;
    }
    if let Some(pattern) = object.get("pattern").and_then(Value::as_str)
        && !Regex::new(pattern).is_ok_and(|regex| regex.is_match(text))
    {
        return false;
    }
    if let Some(format) = object.get("format").and_then(Value::as_str)
        && !string_matches_format(text, format)
    {
        return false;
    }

    true
}

fn string_matches_format(text: &str, format: &str) -> bool {
    match format {
        "uri" | "url" => is_valid_url(text),
        "email" => is_valid_email(text),
        "hostname" => is_valid_hostname(text),
        "ipv4" => text.parse::<std::net::Ipv4Addr>().is_ok(),
        "ipv6" => text.parse::<std::net::Ipv6Addr>().is_ok(),
        _ => true,
    }
}
