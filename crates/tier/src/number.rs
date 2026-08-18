use std::cmp::Ordering;

use serde_json::Number;

const I128_MIN_AS_F64: f64 = -170_141_183_460_469_231_731_687_303_715_884_105_728.0;
const I128_MAX_EXCLUSIVE_AS_F64: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;

#[derive(Debug, Clone, Copy)]
struct DecimalRational {
    numerator: i128,
    denominator: i128,
}

pub(crate) fn compare_json_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    if let (Some(left), Some(right)) = (json_number_as_i128(left), json_number_as_i128(right)) {
        return Some(left.cmp(&right));
    }
    if let Some(left) = json_number_as_i128(left) {
        return compare_i128_to_f64(left, right.as_f64()?);
    }
    if let Some(right) = json_number_as_i128(right) {
        return compare_i128_to_f64(right, left.as_f64()?).map(Ordering::reverse);
    }

    let left = left.as_f64()?;
    let right = right.as_f64()?;
    if !left.is_finite() || !right.is_finite() {
        return None;
    }
    left.partial_cmp(&right)
}

pub(crate) fn json_number_as_i128(value: &Number) -> Option<i128> {
    if let Some(value) = value.as_i64() {
        Some(i128::from(value))
    } else {
        value.as_u64().map(i128::from)
    }
}

#[cfg(feature = "schema")]
pub(crate) fn json_number_is_integer(value: &Number) -> bool {
    json_number_as_i128(value).is_some()
        || value
            .as_f64()
            .is_some_and(|value| value.is_finite() && value.fract() == 0.0)
}

fn compare_i128_to_f64(integer: i128, float: f64) -> Option<Ordering> {
    if !float.is_finite() {
        return None;
    }
    if let Some(float_integer) = whole_f64_to_i128(float) {
        return Some(integer.cmp(&float_integer));
    }

    if float >= I128_MAX_EXCLUSIVE_AS_F64 {
        return Some(Ordering::Less);
    }
    if float < I128_MIN_AS_F64 {
        return Some(Ordering::Greater);
    }

    let float_floor = f64_to_i128_in_range(float.floor())?;
    if integer <= float_floor {
        Some(Ordering::Less)
    } else {
        Some(Ordering::Greater)
    }
}

fn whole_f64_to_i128(value: f64) -> Option<i128> {
    (value.fract() == 0.0)
        .then(|| f64_to_i128_in_range(value))
        .flatten()
}

pub(crate) fn json_number_from_i128(value: i128) -> Option<Number> {
    i64::try_from(value)
        .map(Number::from)
        .or_else(|_| u64::try_from(value).map(Number::from))
        .ok()
}

pub(crate) fn json_number_from_u128(value: u128) -> Option<Number> {
    u64::try_from(value).map(Number::from).ok()
}

#[cfg(feature = "schema")]
pub(crate) fn finite_f64_to_i128(value: f64) -> Option<i128> {
    f64_to_i128_in_range(value)
}

fn f64_to_i128_in_range(value: f64) -> Option<i128> {
    (value.is_finite() && (I128_MIN_AS_F64..I128_MAX_EXCLUSIVE_AS_F64).contains(&value))
        .then_some(value as i128)
}

pub(crate) fn json_number_is_multiple_of(value: &Number, factor: &Number) -> Option<bool> {
    if let (Some(value), Some(factor)) = (json_number_as_i128(value), json_number_as_i128(factor)) {
        return (factor > 0).then_some(value % factor == 0);
    }

    if let (Some(value), Some(factor)) = (
        decimal_rational_from_number(value),
        positive_decimal_rational_from_number(factor),
    ) && let Some(is_multiple) = rational_is_multiple_of(value, factor)
    {
        return Some(is_multiple);
    }

    let value = value.as_f64()?;
    let factor = factor.as_f64()?;
    f64_is_multiple_of(value, factor)
}

#[cfg(feature = "schema")]
pub(crate) fn integer_multiple_step_from_json_number(number: &Number) -> Option<i128> {
    if let Some(integer) = json_number_as_i128(number) {
        return (integer > 0).then_some(integer);
    }

    positive_decimal_rational_from_number(number).and_then(|rational| {
        let divisor = gcd_i128(rational.numerator, rational.denominator);
        rational
            .numerator
            .checked_div(divisor)
            .filter(|step| *step > 0)
    })
}

fn f64_is_multiple_of(value: f64, factor: f64) -> Option<bool> {
    if !value.is_finite() || !factor.is_finite() || !factor.is_normal() || factor <= 0.0 {
        return None;
    }

    let quotient = value / factor;
    let nearest = quotient.round();
    let tolerance = f64::EPSILON * 16.0 * quotient.abs().max(1.0);
    Some((quotient - nearest).abs() <= tolerance)
}

fn rational_is_multiple_of(value: DecimalRational, factor: DecimalRational) -> Option<bool> {
    let dividend = value.numerator.checked_mul(factor.denominator)?;
    let divisor = value.denominator.checked_mul(factor.numerator)?;
    (divisor > 0).then_some(dividend % divisor == 0)
}

fn positive_decimal_rational_from_number(number: &Number) -> Option<DecimalRational> {
    decimal_rational_from_number(number).filter(|rational| rational.numerator > 0)
}

fn decimal_rational_from_number(number: &Number) -> Option<DecimalRational> {
    let text = number.to_string();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (text.as_str(), 0),
    };
    let (negative, mantissa) = match mantissa.strip_prefix('-') {
        Some(mantissa) => (true, mantissa),
        None => (false, mantissa),
    };

    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let unsigned_numerator = digits.parse::<i128>().ok()?;
    let numerator = if negative {
        unsigned_numerator.checked_neg()?
    } else {
        unsigned_numerator
    };

    let fraction_digits = i32::try_from(fraction.len()).ok()?;
    let scale = fraction_digits.checked_sub(exponent)?;
    let rational = if scale <= 0 {
        let multiplier = pow10_i128(scale.unsigned_abs())?;
        DecimalRational {
            numerator: numerator.checked_mul(multiplier)?,
            denominator: 1,
        }
    } else {
        DecimalRational {
            numerator,
            denominator: pow10_i128(u32::try_from(scale).ok()?)?,
        }
    };

    Some(reduce_decimal_rational(rational))
}

fn reduce_decimal_rational(rational: DecimalRational) -> DecimalRational {
    let divisor = gcd_i128(rational.numerator, rational.denominator);
    DecimalRational {
        numerator: rational.numerator / divisor,
        denominator: rational.denominator / divisor,
    }
}

fn pow10_i128(exponent: u32) -> Option<i128> {
    let mut value = 1i128;
    for _ in 0..exponent {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

fn gcd_i128(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs().max(1)
}

#[cfg(all(test, feature = "schema"))]
mod tests {
    use std::cmp::Ordering;

    use super::{
        compare_json_numbers, finite_f64_to_i128, json_number_is_integer,
        json_number_is_multiple_of,
    };

    #[test]
    fn finite_f64_to_i128_rejects_non_finite_and_out_of_range_values() {
        assert_eq!(finite_f64_to_i128(42.0), Some(42));
        assert_eq!(finite_f64_to_i128(-42.0), Some(-42));
        assert_eq!(finite_f64_to_i128(f64::NAN), None);
        assert_eq!(finite_f64_to_i128(f64::INFINITY), None);
        assert_eq!(finite_f64_to_i128(2.0_f64.powi(127)), None);
        assert_eq!(finite_f64_to_i128(-2.0_f64.powi(127) * 2.0), None);
    }

    #[test]
    fn json_number_is_integer_accepts_float_encoded_whole_numbers() {
        let whole = serde_json::json!(1.0);
        let fractional = serde_json::json!(1.5);

        assert!(json_number_is_integer(
            whole.as_number().expect("whole number")
        ));
        assert!(!json_number_is_integer(
            fractional.as_number().expect("fractional number")
        ));
    }

    #[test]
    fn compare_json_numbers_keeps_large_integer_float_comparisons_exact() {
        let integer = serde_json::json!(9_007_199_254_740_993_u64);
        let float = serde_json::json!(9_007_199_254_740_992.0_f64);

        assert_eq!(
            compare_json_numbers(
                integer.as_number().expect("integer"),
                float.as_number().expect("float")
            ),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn json_number_is_multiple_of_preserves_large_integer_precision() {
        let odd = serde_json::Number::from(9_007_199_254_740_993_u64);
        let even = serde_json::Number::from(9_007_199_254_740_994_u64);
        let factor = serde_json::Number::from_f64(2.0).expect("finite factor");

        assert_eq!(json_number_is_multiple_of(&odd, &factor), Some(false));
        assert_eq!(json_number_is_multiple_of(&even, &factor), Some(true));
    }
}
