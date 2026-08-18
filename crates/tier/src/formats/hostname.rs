pub(crate) fn is_valid_hostname(value: &str) -> bool {
    if value.is_empty() || value.len() > 253 {
        return false;
    }

    let hostname = value.strip_suffix('.').unwrap_or(value);
    if hostname.is_empty() {
        return false;
    }

    hostname.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    })
}

#[cfg(test)]
mod tests {
    use super::is_valid_hostname;

    #[test]
    fn validates_hostnames() {
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("example.com."));
        assert!(is_valid_hostname("localhost."));
        assert!(is_valid_hostname("api-1.internal"));
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("."));
        assert!(!is_valid_hostname("-example.com"));
        assert!(!is_valid_hostname("example-.com"));
        assert!(!is_valid_hostname("example..com"));
        assert!(!is_valid_hostname("example.com.."));
    }
}
