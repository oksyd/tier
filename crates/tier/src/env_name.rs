pub(crate) fn normalize_env_prefix(prefix: &str, separator: &str) -> String {
    if prefix.is_empty() {
        return String::new();
    }

    let mut normalized = prefix.to_owned();
    if !separator.is_empty() {
        while normalized.ends_with(separator) {
            normalized.truncate(normalized.len() - separator.len());
        }
    }
    if separator != "_" {
        normalized = normalized.trim_end_matches('_').to_owned();
    }
    normalized
}

#[cfg(feature = "schema")]
pub(crate) fn path_to_env_name(
    path: &str,
    prefix: Option<&str>,
    separator: &str,
    uppercase: bool,
) -> String {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| env_segment_name(segment, uppercase))
        .collect::<Vec<_>>();
    let body = segments.join(separator);

    match prefix {
        Some(prefix) if !prefix.is_empty() => {
            let prefix = normalize_env_prefix(prefix, separator);
            if prefix.is_empty() {
                body
            } else if body.is_empty() {
                prefix
            } else {
                format!("{prefix}{separator}{body}")
            }
        }
        _ => body,
    }
}

#[cfg(feature = "schema")]
fn env_segment_name(segment: &str, uppercase: bool) -> String {
    if segment == "*" {
        "{item}".to_owned()
    } else if segment.starts_with('{') && segment.ends_with('}') {
        segment.to_owned()
    } else if uppercase {
        segment.to_ascii_uppercase()
    } else {
        segment.to_owned()
    }
}
