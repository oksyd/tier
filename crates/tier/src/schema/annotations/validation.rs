use std::cmp::Ordering;

use serde_json::{Map, Number, Value};

use crate::metadata::{ValidationNumber, ValidationRule, ValidationValue};
use crate::number::{
    compare_json_numbers, json_number_as_i128, json_number_from_i128, json_number_from_u128,
};

pub(super) fn apply_validation_schema_keywords(
    object: &mut Map<String, Value>,
    rules: &[ValidationRule],
) {
    apply_validation_schema_keywords_with_policy(object, rules, IncompatibleTypePolicy::Ignore);
}

fn apply_validation_schema_keywords_with_policy(
    object: &mut Map<String, Value>,
    rules: &[ValidationRule],
    incompatible_type_policy: IncompatibleTypePolicy,
) {
    apply_node_validation_schema_keywords(object, rules, incompatible_type_policy);
    apply_combinator_validation_schema_keywords(object, rules);
}

fn apply_node_validation_schema_keywords(
    object: &mut Map<String, Value>,
    rules: &[ValidationRule],
    incompatible_type_policy: IncompatibleTypePolicy,
) {
    for rule in rules {
        match rule {
            ValidationRule::NonEmpty => apply_non_empty(object),
            ValidationRule::Min(value) => {
                if constrain_type(object, TypeConstraint::Numeric, incompatible_type_policy) {
                    insert_number_bound(object, "minimum", value, Bound::Lower);
                }
            }
            ValidationRule::Max(value) => {
                if constrain_type(object, TypeConstraint::Numeric, incompatible_type_policy) {
                    insert_number_bound(object, "maximum", value, Bound::Upper);
                }
            }
            ValidationRule::MinLength(value) => {
                apply_length_bound(object, *value, Bound::Lower, incompatible_type_policy);
            }
            ValidationRule::MaxLength(value) => {
                apply_length_bound(object, *value, Bound::Upper, incompatible_type_policy);
            }
            ValidationRule::MinItems(value) => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["array"]),
                    incompatible_type_policy,
                ) {
                    insert_usize_bound(object, "minItems", *value, Bound::Lower);
                }
            }
            ValidationRule::MaxItems(value) => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["array"]),
                    incompatible_type_policy,
                ) {
                    insert_usize_bound(object, "maxItems", *value, Bound::Upper);
                }
            }
            ValidationRule::MinProperties(value) => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["object"]),
                    incompatible_type_policy,
                ) {
                    insert_usize_bound(object, "minProperties", *value, Bound::Lower);
                }
            }
            ValidationRule::MaxProperties(value) => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["object"]),
                    incompatible_type_policy,
                ) {
                    insert_usize_bound(object, "maxProperties", *value, Bound::Upper);
                }
            }
            ValidationRule::MultipleOf(value) => {
                if constrain_type(object, TypeConstraint::Numeric, incompatible_type_policy) {
                    insert_multiple_of(object, value);
                }
            }
            ValidationRule::Pattern(pattern) => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["string"]),
                    incompatible_type_policy,
                ) {
                    insert_string_constraint(object, "pattern", pattern);
                }
            }
            ValidationRule::UniqueItems => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["array"]),
                    incompatible_type_policy,
                ) {
                    object.insert("uniqueItems".to_owned(), Value::Bool(true));
                }
            }
            ValidationRule::OneOf(values) => insert_enum(object, values),
            ValidationRule::Url => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["string"]),
                    incompatible_type_policy,
                ) {
                    insert_format(object, "uri");
                }
            }
            ValidationRule::Email => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["string"]),
                    incompatible_type_policy,
                ) {
                    insert_format(object, "email");
                }
            }
            ValidationRule::Hostname => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["string"]),
                    incompatible_type_policy,
                ) {
                    insert_format(object, "hostname");
                }
            }
            ValidationRule::IpAddr => {
                if constrain_type(
                    object,
                    TypeConstraint::Named(&["string"]),
                    incompatible_type_policy,
                ) {
                    insert_ip_addr_constraint(object);
                }
            }
            ValidationRule::SocketAddr | ValidationRule::AbsolutePath => {}
        }
    }
}

fn apply_combinator_validation_schema_keywords(
    object: &mut Map<String, Value>,
    rules: &[ValidationRule],
) {
    for keyword in ["allOf", "anyOf", "oneOf"] {
        let Some(children) = object.get_mut(keyword).and_then(Value::as_array_mut) else {
            continue;
        };
        for child in children {
            if let Some(child) = child.as_object_mut() {
                // Union branches must reject incompatible variants; top-level wildcard
                // annotations may legitimately land on schema nodes with mixed shapes.
                apply_validation_schema_keywords_with_policy(
                    child,
                    rules,
                    IncompatibleTypePolicy::MarkUnsatisfiable,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum IncompatibleTypePolicy {
    Ignore,
    MarkUnsatisfiable,
}

#[derive(Clone, Copy)]
enum Bound {
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
enum TypeConstraint {
    Named(&'static [&'static str]),
    Numeric,
}

impl TypeConstraint {
    fn accepts(self, ty: &str) -> bool {
        match self {
            Self::Named(types) => types.contains(&ty),
            Self::Numeric => matches!(ty, "number" | "integer"),
        }
    }

    fn schema_types(self) -> &'static [&'static str] {
        match self {
            Self::Named(types) => types,
            Self::Numeric => &["number"],
        }
    }

    fn schema_value(self) -> Value {
        schema_type_value(self.schema_types())
    }
}

fn apply_non_empty(object: &mut Map<String, Value>) {
    let mut projected = false;
    if schema_type_includes(object, "string") {
        insert_usize_bound(object, "minLength", 1, Bound::Lower);
        projected = true;
    }
    if schema_type_includes(object, "array") {
        insert_usize_bound(object, "minItems", 1, Bound::Lower);
        projected = true;
    }
    if schema_type_includes(object, "object") {
        insert_usize_bound(object, "minProperties", 1, Bound::Lower);
        projected = true;
    }

    if !projected && !object.contains_key("type") && !has_combinator_branches(object) {
        insert_usize_bound(object, "minLength", 1, Bound::Lower);
        insert_usize_bound(object, "minItems", 1, Bound::Lower);
        insert_usize_bound(object, "minProperties", 1, Bound::Lower);
    }
}

fn apply_length_bound(
    object: &mut Map<String, Value>,
    value: usize,
    bound: Bound,
    incompatible_type_policy: IncompatibleTypePolicy,
) {
    if !constrain_type(
        object,
        TypeConstraint::Named(&["string", "array", "object"]),
        incompatible_type_policy,
    ) {
        return;
    }

    if schema_type_includes(object, "string") {
        insert_usize_bound(
            object,
            length_keyword("minLength", "maxLength", bound),
            value,
            bound,
        );
    }
    if schema_type_includes(object, "array") {
        insert_usize_bound(
            object,
            length_keyword("minItems", "maxItems", bound),
            value,
            bound,
        );
    }
    if schema_type_includes(object, "object") {
        insert_usize_bound(
            object,
            length_keyword("minProperties", "maxProperties", bound),
            value,
            bound,
        );
    }
}

fn length_keyword(
    min_keyword: &'static str,
    max_keyword: &'static str,
    bound: Bound,
) -> &'static str {
    match bound {
        Bound::Lower => min_keyword,
        Bound::Upper => max_keyword,
    }
}

fn insert_number_bound(
    object: &mut Map<String, Value>,
    keyword: &str,
    value: &ValidationNumber,
    bound: Bound,
) {
    let ValidationNumber::Finite(candidate) = value else {
        return;
    };
    insert_stricter_number(object, keyword, candidate, bound);
}

fn insert_usize_bound(object: &mut Map<String, Value>, keyword: &str, value: usize, bound: Bound) {
    let Some(candidate) = u128::try_from(value).ok().and_then(json_number_from_u128) else {
        return;
    };
    insert_stricter_number(object, keyword, &candidate, bound);
}

fn insert_stricter_number(
    object: &mut Map<String, Value>,
    keyword: &str,
    candidate: &Number,
    bound: Bound,
) {
    let should_insert = object
        .get(keyword)
        .and_then(Value::as_number)
        .and_then(|existing| compare_json_numbers(candidate, existing))
        .is_none_or(|ordering| match bound {
            Bound::Lower => matches!(ordering, Ordering::Greater),
            Bound::Upper => matches!(ordering, Ordering::Less),
        });

    if should_insert {
        object.insert(keyword.to_owned(), Value::Number(candidate.clone()));
    }
}

fn insert_multiple_of(object: &mut Map<String, Value>, value: &ValidationNumber) {
    let ValidationNumber::Finite(candidate) = value else {
        return;
    };
    if !number_is_positive(candidate) {
        return;
    }
    if let Some(existing) = object.get("multipleOf").and_then(Value::as_number)
        && let Some(combined) = combined_integer_multiple(existing, candidate)
    {
        object.insert("multipleOf".to_owned(), Value::Number(combined));
        return;
    }

    insert_constraint(object, "multipleOf", Value::Number(candidate.clone()));
}

fn insert_enum(object: &mut Map<String, Value>, values: &[ValidationValue]) {
    if values.is_empty() {
        return;
    }
    if has_alternative_branches(object) && !object.contains_key("type") {
        return;
    }
    if schema_accepts_only_null(object) {
        return;
    }

    let mut values = values
        .iter()
        .map(|value| value.0.clone())
        .collect::<Vec<_>>();
    if schema_accepts_null(object) && !values.iter().any(Value::is_null) {
        values.push(Value::Null);
    }
    let values = Value::Array(values);
    insert_constraint(object, "enum", values);
}

fn insert_format(object: &mut Map<String, Value>, format: &str) {
    insert_constraint(object, "format", Value::String(format.to_owned()));
}

fn insert_ip_addr_constraint(object: &mut Map<String, Value>) {
    if matches!(
        object.get("format").and_then(Value::as_str),
        Some("ipv4" | "ipv6")
    ) {
        return;
    }

    insert_constraint(
        object,
        "oneOf",
        Value::Array(vec![
            string_format_schema("ipv4"),
            string_format_schema("ipv6"),
        ]),
    );
}

fn string_format_schema(format: &str) -> Value {
    Value::Object(Map::from_iter([
        ("type".to_owned(), Value::String("string".to_owned())),
        ("format".to_owned(), Value::String(format.to_owned())),
    ]))
}

fn schema_type_includes(object: &Map<String, Value>, expected: &str) -> bool {
    match object.get("type") {
        Some(Value::String(ty)) => ty == expected,
        Some(Value::Array(types)) => types.iter().any(|ty| ty.as_str() == Some(expected)),
        _ => false,
    }
}

fn constrain_type(
    object: &mut Map<String, Value>,
    constraint: TypeConstraint,
    incompatible_type_policy: IncompatibleTypePolicy,
) -> bool {
    let Some(existing) = object.get("type").cloned() else {
        if schema_accepts_only_null(object) {
            return false;
        }
        if has_combinator_branches(object) {
            return false;
        }
        if schema_accepts_null(object) {
            object.insert("type".to_owned(), schema_type_value_with_null(constraint));
            return true;
        }
        object.insert("type".to_owned(), constraint.schema_value());
        return true;
    };

    let Some(existing_types) = schema_type_values(&existing) else {
        return true;
    };

    let accepted_non_null = existing_types
        .iter()
        .copied()
        .filter(|ty| constraint.accepts(ty))
        .collect::<Vec<_>>();
    let accepts_null = existing_types.contains(&"null");
    let mut accepted = accepted_non_null.clone();
    if accepts_null {
        accepted.push("null");
    }

    if accepted.is_empty() {
        if matches!(
            incompatible_type_policy,
            IncompatibleTypePolicy::MarkUnsatisfiable
        ) {
            insert_never_schema(object);
        }
        return false;
    }
    if accepted.len() != existing_types.len() {
        object.insert("type".to_owned(), schema_type_value(&accepted));
    }

    !accepted_non_null.is_empty()
}

fn schema_type_values(value: &Value) -> Option<Vec<&str>> {
    match value {
        Value::String(ty) => Some(vec![ty.as_str()]),
        Value::Array(types) => Some(types.iter().filter_map(Value::as_str).collect()),
        _ => None,
    }
}

fn schema_type_value(types: &[&str]) -> Value {
    match types {
        [ty] => Value::String((*ty).to_owned()),
        _ => Value::Array(
            types
                .iter()
                .map(|ty| Value::String((*ty).to_owned()))
                .collect(),
        ),
    }
}

fn schema_type_value_with_null(constraint: TypeConstraint) -> Value {
    let mut types = constraint.schema_types().to_vec();
    if !types.contains(&"null") {
        types.push("null");
    }
    schema_type_value(&types)
}

fn has_combinator_branches(object: &Map<String, Value>) -> bool {
    ["allOf", "anyOf", "oneOf"]
        .into_iter()
        .any(|keyword| object.get(keyword).and_then(Value::as_array).is_some())
}

fn has_alternative_branches(object: &Map<String, Value>) -> bool {
    ["anyOf", "oneOf"]
        .into_iter()
        .any(|keyword| object.get(keyword).and_then(Value::as_array).is_some())
}

fn schema_accepts_null(object: &Map<String, Value>) -> bool {
    if object
        .get("type")
        .and_then(schema_type_values)
        .is_some_and(|types| !types.contains(&"null"))
    {
        return false;
    }

    if let Some(const_value) = object.get("const") {
        return const_value.is_null();
    }

    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return values.iter().any(Value::is_null);
    }

    schema_type_includes(object, "null")
}

fn schema_accepts_only_null(object: &Map<String, Value>) -> bool {
    let type_values = object.get("type").and_then(schema_type_values);
    let type_allows_null = type_values
        .as_ref()
        .is_none_or(|types| types.contains(&"null"));
    if !type_allows_null {
        return false;
    }

    if let Some(const_value) = object.get("const") {
        return const_value.is_null();
    }

    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return !values.is_empty() && values.iter().all(Value::is_null);
    }

    type_values.is_some_and(|types| !types.is_empty() && types.iter().all(|ty| *ty == "null"))
}

fn insert_never_schema(object: &mut Map<String, Value>) {
    insert_constraint(object, "not", Value::Object(Map::new()));
}

fn number_is_positive(number: &Number) -> bool {
    compare_json_numbers(number, &Number::from(0)).is_some_and(|ordering| ordering.is_gt())
}

fn combined_integer_multiple(existing: &Number, candidate: &Number) -> Option<Number> {
    let existing = json_number_as_i128(existing)?;
    let candidate = json_number_as_i128(candidate)?;
    if existing <= 0 || candidate <= 0 {
        return None;
    }
    let gcd = gcd_i128(existing, candidate);
    existing
        .checked_div(gcd)?
        .checked_mul(candidate)
        .and_then(json_number_from_i128)
}

fn gcd_i128(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.abs()
}

fn insert_string_constraint(object: &mut Map<String, Value>, keyword: &str, value: &str) {
    insert_constraint(object, keyword, Value::String(value.to_owned()));
}

fn insert_constraint(object: &mut Map<String, Value>, keyword: &str, value: Value) {
    match object.get(keyword) {
        None => {
            object.insert(keyword.to_owned(), value);
        }
        Some(existing) if existing == &value => {}
        Some(_) => push_all_of_constraint(object, keyword, value),
    }
}

fn push_all_of_constraint(object: &mut Map<String, Value>, keyword: &str, value: Value) {
    let constraint = Value::Object(Map::from_iter([(keyword.to_owned(), value)]));
    match object.get_mut("allOf").and_then(Value::as_array_mut) {
        Some(all_of) => all_of.push(constraint),
        None => {
            object.insert("allOf".to_owned(), Value::Array(vec![constraint]));
        }
    }
}
