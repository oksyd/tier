use serde_json::Value;

use super::merge::merge_schema_keyword;

pub(crate) fn inlined_schema_ref(schema: &Value, root: &Value) -> Option<Value> {
    let reference = schema.get("$ref").and_then(Value::as_str)?;
    let target = resolve_schema_ref(root, reference)?;
    let mut inlined = target.clone();
    if let (Some(inlined_object), Some(reference_object)) =
        (inlined.as_object_mut(), schema.as_object())
    {
        for (key, value) in reference_object {
            if key != "$ref" {
                merge_schema_keyword(inlined_object, key, value);
            }
        }
    }
    Some(inlined)
}

pub(crate) fn resolve_schema_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}
