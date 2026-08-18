use std::net::{IpAddr, Ipv6Addr};

use super::hostname::is_valid_hostname;
use super::shared::{has_no_space_or_control, has_valid_percent_escapes};

pub(crate) fn is_valid_url(value: &str) -> bool {
    if value.is_empty() || !has_no_space_or_control(value) {
        return false;
    }

    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    if !is_valid_url_scheme(scheme) || rest.is_empty() {
        return false;
    }

    if let Some(authority_and_tail) = rest.strip_prefix("//") {
        if !scheme_allows_authority(scheme) {
            return false;
        }
        return is_valid_hierarchical_url(scheme, authority_and_tail);
    }

    if scheme_requires_authority(scheme) {
        return false;
    }

    is_valid_opaque_or_path_url(scheme, rest)
}

fn is_valid_url_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

fn is_local_resource_scheme(scheme: &str) -> bool {
    scheme.eq_ignore_ascii_case("file") || scheme.eq_ignore_ascii_case("unix")
}

fn scheme_requires_authority(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "ws" | "wss" | "ftp"
    )
}

fn scheme_allows_authority(scheme: &str) -> bool {
    !scheme.eq_ignore_ascii_case("mailto")
}

fn is_valid_opaque_or_path_url(scheme: &str, rest: &str) -> bool {
    if rest.starts_with("//") || !has_valid_percent_escapes(rest) {
        return false;
    }

    if scheme.eq_ignore_ascii_case("mailto") {
        return !rest.starts_with('/');
    }

    true
}

fn is_valid_hierarchical_url(scheme: &str, authority_and_tail: &str) -> bool {
    let split_at = authority_and_tail
        .find(['/', '?', '#'])
        .unwrap_or(authority_and_tail.len());
    let (authority, tail) = authority_and_tail.split_at(split_at);

    if authority.is_empty() {
        return is_local_resource_scheme(scheme)
            && !tail.is_empty()
            && has_valid_percent_escapes(tail);
    }

    if !has_valid_percent_escapes(authority) || !has_valid_percent_escapes(tail) {
        return false;
    }

    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(userinfo, host_port)| {
            if userinfo.is_empty() || userinfo.contains('@') || !is_valid_url_userinfo(userinfo) {
                ""
            } else {
                host_port
            }
        });
    is_valid_url_host_port(host_port)
}

fn is_valid_url_host_port(host_port: &str) -> bool {
    if host_port.is_empty() {
        return false;
    }

    if let Some(ipv6) = host_port.strip_prefix('[') {
        let Some((host, suffix)) = ipv6.split_once(']') else {
            return false;
        };
        if host.parse::<Ipv6Addr>().is_err() {
            return false;
        }
        return suffix.is_empty() || parse_url_port(suffix.strip_prefix(':')).is_some();
    }

    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host, Some(port)),
        Some(_) => return false,
        None => (host_port, None),
    };

    if host.is_empty() || !(host.parse::<IpAddr>().is_ok() || is_valid_hostname(host)) {
        return false;
    }

    port.is_none_or(|port| parse_url_port(Some(port)).is_some())
}

fn parse_url_port(port: Option<&str>) -> Option<u16> {
    let port = port?;
    if port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    port.parse::<u16>().ok()
}

fn is_valid_url_userinfo(value: &str) -> bool {
    value.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '-' | '.'
                    | '_'
                    | '~'
                    | '!'
                    | '$'
                    | '&'
                    | '\''
                    | '('
                    | ')'
                    | '*'
                    | '+'
                    | ','
                    | ';'
                    | '='
                    | ':'
                    | '%'
            )
    })
}

#[cfg(test)]
mod tests {
    use super::is_valid_url;

    #[test]
    fn validates_general_urls_without_external_parser() {
        assert!(is_valid_url("https://example.com:8443/path?q=1#frag"));
        assert!(is_valid_url("HTTPS://example.com"));
        assert!(is_valid_url("https://example.com./path"));
        assert!(is_valid_url("mailto:ops@example.com"));
        assert!(is_valid_url(
            "urn:uuid:123e4567-e89b-12d3-a456-426614174000"
        ));
        assert!(is_valid_url("data:text/plain,hello"));
        assert!(is_valid_url("custom:/resource/path"));
        assert!(is_valid_url("file:///var/lib/app/config.toml"));
        assert!(is_valid_url("unix:/run/app.sock"));
        assert!(is_valid_url("http://[::1]:8080/health"));
        assert!(!is_valid_url("https://example.com:%zz"));
        assert!(!is_valid_url("https://example.com:99999"));
        assert!(!is_valid_url("1http://example.com"));
        assert!(!is_valid_url("https://"));
        assert!(!is_valid_url("http:example.com"));
        assert!(!is_valid_url("mailto://ops@example.com"));
        assert!(!is_valid_url("https://example .com"));
        assert!(!is_valid_url("https://example.com../path"));
    }
}
