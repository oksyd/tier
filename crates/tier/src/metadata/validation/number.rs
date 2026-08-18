use std::fmt::{self, Display, Formatter};

use super::super::ValidationNumber;
use crate::number::{json_number_from_i128, json_number_from_u128};

impl ValidationNumber {
    /// Returns the numeric value as `f64`, when representable.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Finite(number) => number.as_f64(),
            Self::Invalid(_) => None,
        }
    }

    /// Returns `true` when the bound is a finite JSON-compatible number.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        matches!(self, Self::Finite(_))
    }

    /// Returns the bound rendered as a JSON value.
    #[must_use]
    pub fn as_json_value(&self) -> serde_json::Value {
        match self {
            Self::Finite(number) => serde_json::Value::Number(number.clone()),
            Self::Invalid(value) => serde_json::Value::String(value.clone()),
        }
    }
}

impl Display for ValidationNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finite(number) => Display::fmt(number, f),
            Self::Invalid(value) => Display::fmt(value, f),
        }
    }
}

impl From<i8> for ValidationNumber {
    fn from(value: i8) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<i16> for ValidationNumber {
    fn from(value: i16) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<i32> for ValidationNumber {
    fn from(value: i32) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<i64> for ValidationNumber {
    fn from(value: i64) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<i128> for ValidationNumber {
    fn from(value: i128) -> Self {
        json_number_from_i128(value).map_or_else(|| Self::Invalid(value.to_string()), Self::Finite)
    }
}

impl From<isize> for ValidationNumber {
    fn from(value: isize) -> Self {
        i128::try_from(value).map_or_else(|_| Self::Invalid(value.to_string()), Self::from)
    }
}

impl From<u8> for ValidationNumber {
    fn from(value: u8) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<u16> for ValidationNumber {
    fn from(value: u16) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<u32> for ValidationNumber {
    fn from(value: u32) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<u64> for ValidationNumber {
    fn from(value: u64) -> Self {
        Self::Finite(serde_json::Number::from(value))
    }
}

impl From<u128> for ValidationNumber {
    fn from(value: u128) -> Self {
        json_number_from_u128(value).map_or_else(|| Self::Invalid(value.to_string()), Self::Finite)
    }
}

impl From<usize> for ValidationNumber {
    fn from(value: usize) -> Self {
        u128::try_from(value).map_or_else(|_| Self::Invalid(value.to_string()), Self::from)
    }
}

impl From<f32> for ValidationNumber {
    fn from(value: f32) -> Self {
        match serde_json::Number::from_f64(f64::from(value)) {
            Some(number) => Self::Finite(number),
            None => Self::Invalid(value.to_string()),
        }
    }
}

impl From<f64> for ValidationNumber {
    fn from(value: f64) -> Self {
        match serde_json::Number::from_f64(value) {
            Some(number) => Self::Finite(number),
            None => Self::Invalid(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationNumber;

    #[test]
    fn validation_number_accepts_json_representable_wide_integers() {
        assert!(matches!(
            ValidationNumber::from(i128::from(u64::MAX)),
            ValidationNumber::Finite(_)
        ));
        assert!(matches!(
            ValidationNumber::from(u128::from(u64::MAX)),
            ValidationNumber::Finite(_)
        ));
    }

    #[test]
    fn validation_number_marks_unrepresentable_wide_integers_invalid() {
        assert!(matches!(
            ValidationNumber::from(i128::from(i64::MIN) - 1),
            ValidationNumber::Invalid(_)
        ));
        assert!(matches!(
            ValidationNumber::from(u128::from(u64::MAX) + 1),
            ValidationNumber::Invalid(_)
        ));
    }
}
