use std::str::FromStr;

use serde::de::{Visitor, value::Error as ValueDeError};
use serde_json::Value;

use super::model::CoercingDeserializer;

pub(super) trait FiniteFloat: FromStr + Copy {
    fn is_finite(self) -> bool;
}

impl FiniteFloat for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl FiniteFloat for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}

pub(super) fn parse_bool(raw: &str) -> Option<bool> {
    let raw = raw.trim();
    if raw.eq_ignore_ascii_case("true")
        || raw.eq_ignore_ascii_case("yes")
        || raw.eq_ignore_ascii_case("y")
        || raw.eq_ignore_ascii_case("on")
        || raw == "1"
    {
        return Some(true);
    }

    if raw.eq_ignore_ascii_case("false")
        || raw.eq_ignore_ascii_case("no")
        || raw.eq_ignore_ascii_case("n")
        || raw.eq_ignore_ascii_case("off")
        || raw == "0"
    {
        return Some(false);
    }

    None
}

pub(super) fn deserialize_integer<'de, 'a, V, T, F>(
    de: &CoercingDeserializer<'a>,
    visitor: V,
    visit: F,
) -> Result<V::Value, ValueDeError>
where
    V: Visitor<'de>,
    T: FromStr,
    F: FnOnce(V, T) -> Result<V::Value, ValueDeError>,
{
    deserialize_number(de, visitor, visit)
}

pub(super) fn deserialize_float<'de, 'a, V, T, F>(
    de: &CoercingDeserializer<'a>,
    visitor: V,
    visit: F,
) -> Result<V::Value, ValueDeError>
where
    V: Visitor<'de>,
    T: FiniteFloat,
    F: FnOnce(V, T) -> Result<V::Value, ValueDeError>,
{
    if let Some(raw) = de.coercible_string() {
        let value = parse_finite_float(raw.trim().parse::<T>(), || {
            de.invalid_string_type(raw, &visitor)
        })?;
        return visit(visitor, value);
    }

    match de.value {
        Value::Number(number) => {
            let value = parse_finite_float(number.to_string().parse::<T>(), || {
                de.invalid_type(&visitor)
            })?;
            visit(visitor, value)
        }
        _ => Err(de.invalid_type(&visitor)),
    }
}

fn parse_finite_float<T, E>(
    parsed: Result<T, E>,
    error: impl Fn() -> ValueDeError,
) -> Result<T, ValueDeError>
where
    T: FiniteFloat,
{
    let value = parsed.map_err(|_| error())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error())
    }
}

fn deserialize_number<'de, 'a, V, T, F>(
    de: &CoercingDeserializer<'a>,
    visitor: V,
    visit: F,
) -> Result<V::Value, ValueDeError>
where
    V: Visitor<'de>,
    T: FromStr,
    F: FnOnce(V, T) -> Result<V::Value, ValueDeError>,
{
    if let Some(raw) = de.coercible_string() {
        return raw
            .trim()
            .parse::<T>()
            .map_err(|_| de.invalid_string_type(raw, &visitor))
            .and_then(|value| visit(visitor, value));
    }

    match de.value {
        Value::Number(number) => number
            .to_string()
            .parse::<T>()
            .map_err(|_| de.invalid_type(&visitor))
            .and_then(|value| visit(visitor, value)),
        _ => Err(de.invalid_type(&visitor)),
    }
}
