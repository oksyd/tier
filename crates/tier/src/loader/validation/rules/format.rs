use std::collections::BTreeSet;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

use regex::Regex;
use serde_json::Value;

use crate::error::ValidationError;
use crate::metadata::ValidationRule;

use super::super::error::validation_error;
use crate::formats::{is_valid_email, is_valid_hostname, is_valid_url};

pub(super) fn validate_pattern(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    pattern: &str,
) -> Option<ValidationError> {
    let Some(value) = actual.as_str() else {
        return Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            "must be a string to apply pattern validation",
            Some(Value::String(pattern.to_owned())),
        ));
    };
    let regex = match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => {
            return Some(validation_error(
                path,
                actual,
                rule,
                secret_paths,
                &format!("declared pattern must be a valid regex: {error}"),
                Some(Value::String(pattern.to_owned())),
            ));
        }
    };
    (!regex.is_match(value)).then(|| {
        validation_error(
            path,
            actual,
            rule,
            secret_paths,
            &format!("must match pattern {pattern:?}"),
            Some(Value::String(pattern.to_owned())),
        )
    })
}

pub(super) fn validate_hostname(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be a hostname string",
        "must be a valid hostname",
        is_valid_hostname,
    )
}

pub(super) fn validate_url(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be a URL string",
        "must be a valid URL",
        is_valid_url,
    )
}

pub(super) fn validate_email(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be an email address string",
        "must be a valid email address",
        is_valid_email,
    )
}

pub(super) fn validate_ip_addr(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be an IP address string",
        "must be a valid IP address",
        |value| value.parse::<IpAddr>().is_ok(),
    )
}

pub(super) fn validate_socket_addr(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be a socket address string",
        "must be a valid socket address",
        |value| value.parse::<SocketAddr>().is_ok(),
    )
}

pub(super) fn validate_absolute_path(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
) -> Option<ValidationError> {
    validate_string_format(
        path,
        actual,
        rule,
        secret_paths,
        "must be a filesystem path string",
        "must be an absolute filesystem path",
        |value| Path::new(value).is_absolute(),
    )
}

fn validate_string_format(
    path: &str,
    actual: &Value,
    rule: &ValidationRule,
    secret_paths: &BTreeSet<String>,
    type_message: &'static str,
    invalid_message: &'static str,
    is_valid: impl FnOnce(&str) -> bool,
) -> Option<ValidationError> {
    let Some(value) = actual.as_str() else {
        return Some(validation_error(
            path,
            actual,
            rule,
            secret_paths,
            type_message,
            None,
        ));
    };

    (!is_valid(value))
        .then(|| validation_error(path, actual, rule, secret_paths, invalid_message, None))
}
