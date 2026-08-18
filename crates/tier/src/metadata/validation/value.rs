use std::fmt::{self, Display, Formatter};

use super::super::ValidationValue;
use crate::number::{json_number_from_i128, json_number_from_u128};

impl Display for ValidationValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            serde_json::Value::String(value) => write!(f, "{value:?}"),
            value => Display::fmt(value, f),
        }
    }
}

impl From<bool> for ValidationValue {
    fn from(value: bool) -> Self {
        Self(serde_json::Value::Bool(value))
    }
}

impl From<String> for ValidationValue {
    fn from(value: String) -> Self {
        Self(serde_json::Value::String(value))
    }
}

impl From<&str> for ValidationValue {
    fn from(value: &str) -> Self {
        Self(serde_json::Value::String(value.to_owned()))
    }
}

impl From<f32> for ValidationValue {
    fn from(value: f32) -> Self {
        match serde_json::Number::from_f64(f64::from(value)) {
            Some(number) => Self(serde_json::Value::Number(number)),
            None => Self(serde_json::Value::String(value.to_string())),
        }
    }
}

impl From<f64> for ValidationValue {
    fn from(value: f64) -> Self {
        match serde_json::Number::from_f64(value) {
            Some(number) => Self(serde_json::Value::Number(number)),
            None => Self(serde_json::Value::String(value.to_string())),
        }
    }
}

fn from_json_number_or_string(
    number: Option<serde_json::Number>,
    fallback: String,
) -> ValidationValue {
    number.map_or_else(
        || ValidationValue(serde_json::Value::String(fallback)),
        |number| ValidationValue(serde_json::Value::Number(number)),
    )
}

macro_rules! impl_validation_value_from_signed_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for ValidationValue {
                fn from(value: $ty) -> Self {
                    from_json_number_or_string(
                        json_number_from_i128(i128::from(value)),
                        value.to_string(),
                    )
                }
            }
        )*
    };
}

macro_rules! impl_validation_value_from_unsigned_number {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for ValidationValue {
                fn from(value: $ty) -> Self {
                    from_json_number_or_string(
                        json_number_from_u128(u128::from(value)),
                        value.to_string(),
                    )
                }
            }
        )*
    };
}

impl_validation_value_from_signed_number!(i8, i16, i32, i64);
impl_validation_value_from_unsigned_number!(u8, u16, u32, u64);

impl From<i128> for ValidationValue {
    fn from(value: i128) -> Self {
        from_json_number_or_string(json_number_from_i128(value), value.to_string())
    }
}

impl From<isize> for ValidationValue {
    fn from(value: isize) -> Self {
        from_json_number_or_string(
            i128::try_from(value).ok().and_then(json_number_from_i128),
            value.to_string(),
        )
    }
}

impl From<u128> for ValidationValue {
    fn from(value: u128) -> Self {
        from_json_number_or_string(json_number_from_u128(value), value.to_string())
    }
}

impl From<usize> for ValidationValue {
    fn from(value: usize) -> Self {
        from_json_number_or_string(
            u128::try_from(value).ok().and_then(json_number_from_u128),
            value.to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::ValidationValue;

    #[test]
    fn validation_value_accepts_json_representable_wide_integers() {
        assert!(matches!(
            ValidationValue::from(i128::from(u64::MAX)).0,
            Value::Number(_)
        ));
        assert!(matches!(
            ValidationValue::from(u128::from(u64::MAX)).0,
            Value::Number(_)
        ));
    }

    #[test]
    fn validation_value_falls_back_to_string_for_unrepresentable_wide_integers() {
        assert!(matches!(
            ValidationValue::from(i128::from(i64::MIN) - 1).0,
            Value::String(_)
        ));
        assert!(matches!(
            ValidationValue::from(u128::from(u64::MAX) + 1).0,
            Value::String(_)
        ));
    }
}
