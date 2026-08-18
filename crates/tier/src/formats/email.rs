use std::net::{IpAddr, Ipv4Addr};

use super::hostname::is_valid_hostname;

pub(crate) fn is_valid_email(value: &str) -> bool {
    if value.is_empty() || value.contains(char::is_whitespace) || value.matches('@').count() != 1 {
        return false;
    }

    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };

    if local.is_empty() || domain.is_empty() || !is_valid_email_local_part(local) {
        return false;
    }

    if let Some(ip_literal) = domain
        .strip_prefix('[')
        .and_then(|domain| domain.strip_suffix(']'))
    {
        return ip_literal.parse::<IpAddr>().is_ok();
    }

    domain.parse::<Ipv4Addr>().is_err() && is_valid_hostname(domain)
}

fn is_valid_email_local_part(value: &str) -> bool {
    value.split('.').all(|atom| {
        !atom.is_empty()
            && atom.chars().all(|ch| {
                ch.is_ascii_alphanumeric()
                    || matches!(
                        ch,
                        '!' | '#'
                            | '$'
                            | '%'
                            | '&'
                            | '\''
                            | '*'
                            | '+'
                            | '/'
                            | '='
                            | '?'
                            | '^'
                            | '_'
                            | '`'
                            | '{'
                            | '|'
                            | '}'
                            | '~'
                            | '-'
                    )
            })
    })
}

#[cfg(test)]
mod tests {
    use super::is_valid_email;

    #[test]
    fn validates_email_addresses() {
        assert!(is_valid_email("ops@example.com"));
        assert!(is_valid_email("ops@example.com."));
        assert!(is_valid_email("first.last+tag@example.co.uk"));
        assert!(is_valid_email("ops@[127.0.0.1]"));
        assert!(!is_valid_email("ops@."));
        assert!(!is_valid_email("ops@127.0.0.1"));
        assert!(!is_valid_email("ops@@example.com"));
        assert!(!is_valid_email(".ops@example.com"));
        assert!(!is_valid_email("ops@example..com"));
    }
}
