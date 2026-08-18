use std::collections::BTreeSet;

pub(crate) fn dynamic_object_placeholder(reserved: &BTreeSet<String>) -> String {
    for candidate in ["{item}", "{key}", "{entry}", "{value}"] {
        if !reserved.contains(candidate) {
            return candidate.to_owned();
        }
    }

    let mut index = 0usize;
    loop {
        let candidate = format!("{{item_{index}}}");
        if !reserved.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}
