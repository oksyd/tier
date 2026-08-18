use super::super::path::invalid_path_key_message;

pub(super) struct DerivedEnvPathError {
    pub(super) path: String,
    pub(super) message: String,
}

pub(super) fn path_for_env_var(
    key: &str,
    prefix: Option<&str>,
    separator: &str,
    lowercase_segments: bool,
) -> Result<Option<String>, DerivedEnvPathError> {
    let remainder = if let Some(prefix) = prefix {
        let normalized = crate::env_name::normalize_env_prefix(prefix, separator);
        if normalized.is_empty() {
            key
        } else {
            if key == normalized {
                return Ok(None);
            }
            let Some(remainder) = key.strip_prefix(&normalized) else {
                return Ok(None);
            };
            let boundary = if prefix.ends_with(separator) && !separator.is_empty() {
                PrefixBoundary::SeparatorOnly
            } else {
                PrefixBoundary::Flexible
            };
            let Some(remainder) = parse_prefixed_env_remainder(remainder, separator, boundary)
            else {
                return Ok(None);
            };
            remainder
        }
    } else {
        key
    };

    if remainder.is_empty() {
        return Ok(None);
    }

    let mut segments = Vec::new();
    for segment in remainder.split(separator) {
        if segment.is_empty() {
            return Ok(None);
        }
        let segment = if lowercase_segments {
            segment.to_ascii_lowercase()
        } else {
            segment.to_owned()
        };
        if let Some(message) = invalid_path_key_message(&segment) {
            let mut path = segments.join(".");
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(&segment);
            return Err(DerivedEnvPathError {
                path,
                message: format!(
                    "environment variable segments must not contain reserved path syntax: {message}"
                ),
            });
        }
        segments.push(segment);
    }

    Ok(Some(segments.join(".")))
}

#[derive(Clone, Copy)]
enum PrefixBoundary {
    SeparatorOnly,
    Flexible,
}

fn parse_prefixed_env_remainder<'a>(
    remainder: &'a str,
    separator: &str,
    boundary: PrefixBoundary,
) -> Option<&'a str> {
    let remainder = match boundary {
        PrefixBoundary::SeparatorOnly => remainder.strip_prefix(separator)?,
        PrefixBoundary::Flexible => {
            if let Some(stripped) = remainder.strip_prefix(separator) {
                stripped
            } else if separator == "__" {
                remainder.strip_prefix('_')?
            } else {
                return None;
            }
        }
    };

    (!remainder.is_empty()).then_some(remainder)
}
