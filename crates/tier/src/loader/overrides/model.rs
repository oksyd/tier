use std::collections::BTreeSet;

use serde_json::Value;

pub(in crate::loader) struct ParsedOverride {
    pub(in crate::loader) value: Value,
    pub(in crate::loader) string_coercion_suffixes: BTreeSet<String>,
}
