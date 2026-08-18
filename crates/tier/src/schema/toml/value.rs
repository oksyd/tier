use serde_json::Value;

pub(super) fn start_toml_section(output: &mut String) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
}

pub(super) fn is_nested_toml_value(value: &Value) -> bool {
    match value {
        Value::Object(_) => true,
        Value::Array(items) => !items.is_empty() && items.iter().all(Value::is_object),
        _ => false,
    }
}

pub(super) fn toml_inline_value(value: &Value) -> String {
    match value {
        Value::Null => toml_string("<unset>"),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => toml_string(string),
        Value::Array(items) => {
            let rendered = items
                .iter()
                .map(toml_inline_value)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{rendered}]")
        }
        Value::Object(map) => {
            let rendered = map
                .iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| format!("{} = {}", toml_key(key), toml_inline_value(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {rendered} }}")
        }
    }
}

pub(super) fn toml_table_name(path: &str) -> String {
    path.split('.')
        .filter(|segment| !segment.is_empty())
        .map(toml_key)
        .collect::<Vec<_>>()
        .join(".")
}

pub(super) fn toml_key(segment: &str) -> String {
    if segment
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        segment.to_owned()
    } else {
        toml_string(segment)
    }
}

fn toml_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            control if control.is_control() => {
                let code = u32::from(control);
                output.push_str(&format!("\\u{:04X}", code));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}
