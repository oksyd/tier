use serde_json::{Map, Value};

pub(crate) fn keyword_u64(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(Value::as_u64)
}

pub(crate) fn usize_saturating(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

pub(crate) fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

pub(crate) fn len_at_least(len: usize, min: u64) -> bool {
    match u64::try_from(len) {
        Ok(len) => len >= min,
        Err(_) => true,
    }
}

pub(crate) fn len_at_most(len: usize, max: u64) -> bool {
    match u64::try_from(len) {
        Ok(len) => len <= max,
        Err(_) => false,
    }
}

pub(crate) fn len_less_than(len: usize, max: u64) -> bool {
    match u64::try_from(len) {
        Ok(len) => len < max,
        Err(_) => false,
    }
}

pub(crate) fn remaining_required(min: u64, existing: usize) -> usize {
    usize_saturating(min.saturating_sub(usize_to_u64_saturating(existing)))
}

pub(crate) fn available_slots(max: Option<u64>, existing: usize) -> usize {
    max.map_or(usize::MAX, usize_saturating)
        .saturating_sub(existing)
}

#[cfg(test)]
mod tests {
    use super::{
        available_slots, len_at_least, len_at_most, len_less_than, remaining_required,
        usize_saturating,
    };

    #[test]
    fn large_json_schema_counts_are_saturated_explicitly() {
        assert_eq!(usize_saturating(u64::MAX), usize::MAX);
        assert_eq!(remaining_required(u64::MAX, 0), usize::MAX);
        assert_eq!(available_slots(Some(u64::MAX), 0), usize::MAX);
    }

    #[test]
    fn length_comparisons_do_not_depend_on_lossy_casts() {
        assert!(len_at_least(usize::MAX, 0));
        assert!(len_at_most(0, u64::MAX));
        assert!(!len_at_most(usize::MAX, 0));
        assert!(!len_less_than(0, 0));
    }
}
