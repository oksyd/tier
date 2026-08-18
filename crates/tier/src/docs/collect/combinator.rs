use serde_json::Value;

pub(super) const COMBINATOR_KEYWORDS: [&str; 3] = ["allOf", "anyOf", "oneOf"];

pub(super) fn branch_is_null(schema: &Value) -> bool {
    schema
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|ty| ty == "null")
}

pub(super) fn branch_required(keyword: &str, parent_required: bool) -> bool {
    keyword == "allOf" && parent_required
}
