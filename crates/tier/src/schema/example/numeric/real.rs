use serde_json::{Number, Value};

use super::bounds::{NumericBound, lower_bound, upper_bound};
use super::number_matches_numeric_schema;

pub(in crate::schema::example) fn fallback_number_example(
    object: &serde_json::Map<String, Value>,
    accepts: impl Fn(&Number) -> bool,
) -> Option<Number> {
    let mut candidates = Vec::new();
    extend_multiple_number_candidates(&mut candidates, object);
    extend_bounded_number_candidates(&mut candidates, object);

    candidates.into_iter().find_map(|candidate| {
        serde_json::Number::from_f64(candidate)
            .filter(|number| number_matches_numeric_schema(number, object) && accepts(number))
    })
}

fn extend_bounded_number_candidates(
    candidates: &mut Vec<f64>,
    object: &serde_json::Map<String, Value>,
) {
    let lower = lower_bound(object);
    let upper = upper_bound(object);

    push_number_candidate(candidates, 0.0);
    if let Some(lower) = lower {
        push_number_candidate(candidates, inclusive_candidate_above(lower, upper));
    }
    if let Some(upper) = upper {
        push_number_candidate(candidates, inclusive_candidate_below(upper, lower));
    }
    if let (Some(lower), Some(upper)) = (lower, upper)
        && lower.value < upper.value
    {
        push_number_candidate(candidates, lower.value + (upper.value - lower.value) / 2.0);
    }
}

fn extend_multiple_number_candidates(
    candidates: &mut Vec<f64>,
    object: &serde_json::Map<String, Value>,
) {
    let Some(step) = object
        .get("multipleOf")
        .and_then(Value::as_f64)
        .filter(|step| step.is_normal() && *step > 0.0)
    else {
        return;
    };

    let lower = lower_bound(object);
    let upper = upper_bound(object);

    if let Some(lower) = lower {
        let mut candidate = (lower.value / step).ceil() * step;
        if lower.exclusive && candidate <= lower.value {
            candidate += step;
        }
        push_number_candidate_window(candidates, candidate, step);
    }

    if let Some(upper) = upper {
        let mut candidate = (upper.value / step).floor() * step;
        if upper.exclusive && candidate >= upper.value {
            candidate -= step;
        }
        push_number_candidate_window(candidates, candidate, -step);
    }

    push_number_candidate_window(candidates, 0.0, step);
}

fn push_number_candidate_window(candidates: &mut Vec<f64>, start: f64, step: f64) {
    for index in 0..=4 {
        push_number_candidate(candidates, start + step * f64::from(index));
    }
}

fn push_number_candidate(candidates: &mut Vec<f64>, candidate: f64) {
    if candidate.is_finite()
        && !candidates
            .iter()
            .any(|existing| existing.to_bits() == candidate.to_bits())
    {
        candidates.push(candidate);
    }
}

fn inclusive_candidate_above(lower: NumericBound, upper: Option<NumericBound>) -> f64 {
    if !lower.exclusive {
        return lower.value;
    }

    upper.filter(|upper| lower.value < upper.value).map_or_else(
        || lower.value + lower.value.abs().max(1.0) * f64::EPSILON * 16.0,
        |upper| lower.value + (upper.value - lower.value) / 2.0,
    )
}

fn inclusive_candidate_below(upper: NumericBound, lower: Option<NumericBound>) -> f64 {
    if !upper.exclusive {
        return upper.value;
    }

    lower.filter(|lower| lower.value < upper.value).map_or_else(
        || upper.value - upper.value.abs().max(1.0) * f64::EPSILON * 16.0,
        |lower| upper.value - (upper.value - lower.value) / 2.0,
    )
}
