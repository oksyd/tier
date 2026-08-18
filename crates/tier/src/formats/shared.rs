pub(super) fn has_valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let Some(first) = bytes.get(index + 1) else {
                return false;
            };
            let Some(second) = bytes.get(index + 2) else {
                return false;
            };
            if !first.is_ascii_hexdigit() || !second.is_ascii_hexdigit() {
                return false;
            }
            index += 3;
            continue;
        }
        index += 1;
    }
    true
}

pub(super) fn has_no_space_or_control(value: &str) -> bool {
    value
        .chars()
        .all(|ch| !ch.is_whitespace() && !ch.is_control())
}
