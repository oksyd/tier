mod external;
mod macro_helpers;
mod pattern;
mod value;

pub(crate) use self::external::{
    ArrayIndexSegment, ExternalPathSegment, checked_array_len_for_index,
    classify_array_index_segment, is_array_index_segment, normalize_external_path,
    normalize_external_path_with_explicit_arrays, parse_array_index_segment, parse_external_path,
    render_external_path, render_path_with_explicit_array_segments,
};
pub use self::macro_helpers::{PathPatternItem, normalize_macro_path, pattern_item_ref};
pub(crate) use self::pattern::{
    canonicalize_path_with_aliases, concrete_paths_overlap, direct_child_array_index, join_path,
    normalize_path, path_child_segments, path_is_at_or_below, path_matches_pattern,
    path_overlaps_pattern, path_segments, path_starts_with_pattern,
};
pub(crate) use self::value::{collect_diff_paths, collect_paths, get_value_at_path, redact_value};

/// Builds a compile-time checked dot path from a config type.
///
/// This macro keeps field names refactor-safe for runtime APIs that still
/// accept string paths, such as manual metadata, validators, `env_decoder`,
/// and report lookup helpers.
///
/// ```rust
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct DbConfig {
///     token: String,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     db: DbConfig,
/// }
///
/// assert_eq!(tier::path!(AppConfig.db.token), "db.token");
/// ```
#[macro_export]
macro_rules! path {
    ($root:ident $(:: $root_tail:ident)* . $segment:tt $(. $rest:tt)*) => {{
        let _ = |__tier_value: &$root $(:: $root_tail)*| {
            let _ = &$crate::__tier_path_check!((__tier_value).$segment $(. $rest)*);
        };
        $crate::path::normalize_macro_path(stringify!($segment $(. $rest)*))
    }};
}

/// Builds a compile-time checked wildcard path from a config type.
///
/// Use this macro for collection item paths such as `services.*.token`.
/// Wildcard segments are type-checked against common collection containers
/// like `Vec<T>`, arrays, maps, and sets.
///
/// ```rust
/// use std::collections::BTreeMap;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct ServiceConfig {
///     token: String,
/// }
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     services: BTreeMap<String, ServiceConfig>,
/// }
///
/// assert_eq!(tier::path_pattern!(AppConfig.services.*.token), "services.*.token");
/// ```
#[macro_export]
macro_rules! path_pattern {
    ($root:ident $(:: $root_tail:ident)* . $segment:tt $(. $rest:tt)*) => {{
        let _ = |__tier_value: &$root $(:: $root_tail)*| {
            $crate::__tier_path_pattern_check!((__tier_value).$segment $(. $rest)*);
        };
        $crate::path::normalize_macro_path(stringify!($segment $(. $rest)*))
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tier_path_check {
    (($value:expr) . $segment:tt) => {
        $value.$segment
    };
    (($value:expr) . $segment:tt $(. $rest:tt)+) => {
        $crate::__tier_path_check!(($value.$segment) $(. $rest)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tier_path_pattern_check {
    (($value:expr) . *) => {
        if false {
            let _ = $crate::path::pattern_item_ref($value);
        }
    };
    (($value:expr) . * $(. $rest:tt)+) => {
        if false {
            if let Some(__tier_item) = $crate::path::pattern_item_ref($value) {
                $crate::__tier_path_pattern_check!((__tier_item) $(. $rest)+);
            }
        }
    };
    (($value:expr) . $segment:tt) => {
        if false {
            let _ = &$value.$segment;
        }
    };
    (($value:expr) . $segment:tt $(. $rest:tt)+) => {
        $crate::__tier_path_pattern_check!((&$value.$segment) $(. $rest)+);
    };
}
