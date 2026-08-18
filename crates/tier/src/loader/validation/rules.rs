use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::ValidationRule;

mod collection;
mod format;
mod numeric;

pub(super) fn validate_declared_rule(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    match rule {
        ValidationRule::NonEmpty => {
            collection::validate_non_empty(path, actual, rule, secret_paths)
        }
        ValidationRule::Min(min) => numeric::validate_min(path, actual, rule, secret_paths, min),
        ValidationRule::Max(max) => numeric::validate_max(path, actual, rule, secret_paths, max),
        ValidationRule::MinLength(min) => {
            collection::validate_min_length(path, actual, rule, secret_paths, *min)
        }
        ValidationRule::MaxLength(max) => {
            collection::validate_max_length(path, actual, rule, secret_paths, *max)
        }
        ValidationRule::MinItems(min) => {
            collection::validate_min_items(path, actual, rule, secret_paths, *min)
        }
        ValidationRule::MaxItems(max) => {
            collection::validate_max_items(path, actual, rule, secret_paths, *max)
        }
        ValidationRule::MinProperties(min) => {
            collection::validate_min_properties(path, actual, rule, secret_paths, *min)
        }
        ValidationRule::MaxProperties(max) => {
            collection::validate_max_properties(path, actual, rule, secret_paths, *max)
        }
        ValidationRule::MultipleOf(factor) => {
            numeric::validate_multiple_of(path, actual, rule, secret_paths, factor)
        }
        ValidationRule::Pattern(pattern) => {
            format::validate_pattern(path, actual, rule, secret_paths, pattern)
        }
        ValidationRule::UniqueItems => {
            collection::validate_unique_items(path, actual, rule, secret_paths)
        }
        ValidationRule::OneOf(values) => {
            collection::validate_one_of(path, actual, rule, secret_paths, values)
        }
        ValidationRule::Hostname => format::validate_hostname(path, actual, rule, secret_paths),
        ValidationRule::Url => format::validate_url(path, actual, rule, secret_paths),
        ValidationRule::Email => format::validate_email(path, actual, rule, secret_paths),
        ValidationRule::IpAddr => format::validate_ip_addr(path, actual, rule, secret_paths),
        ValidationRule::SocketAddr => {
            format::validate_socket_addr(path, actual, rule, secret_paths)
        }
        ValidationRule::AbsolutePath => {
            format::validate_absolute_path(path, actual, rule, secret_paths)
        }
    }
}
