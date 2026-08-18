use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::BuildHasher;
use std::sync::Arc;

/// Hidden helper trait used by [`path_pattern!`](macro@crate::path_pattern).
#[doc(hidden)]
pub trait PathPatternItem {
    /// Item type yielded by the collection-like path segment.
    type Item;
}

impl<T> PathPatternItem for Vec<T> {
    type Item = T;
}

impl<C> PathPatternItem for Option<C>
where
    C: PathPatternItem,
{
    type Item = C::Item;
}

impl<C> PathPatternItem for Box<C>
where
    C: PathPatternItem,
{
    type Item = C::Item;
}

impl<C> PathPatternItem for Arc<C>
where
    C: PathPatternItem,
{
    type Item = C::Item;
}

impl<T, const N: usize> PathPatternItem for [T; N] {
    type Item = T;
}

impl<K, V> PathPatternItem for BTreeMap<K, V> {
    type Item = V;
}

impl<K, V, S> PathPatternItem for HashMap<K, V, S>
where
    S: BuildHasher,
{
    type Item = V;
}

impl<T> PathPatternItem for BTreeSet<T> {
    type Item = T;
}

impl<T, S> PathPatternItem for HashSet<T, S>
where
    S: BuildHasher,
{
    type Item = T;
}

/// Hidden helper used by [`path_pattern!`](macro@crate::path_pattern).
#[doc(hidden)]
#[must_use]
pub fn pattern_item_ref<C>(_: &C) -> Option<&C::Item>
where
    C: PathPatternItem,
{
    None
}

/// Hidden helper used by path macros to normalize `stringify!` output.
#[doc(hidden)]
#[must_use]
pub fn normalize_macro_path(path: &str) -> String {
    path.split('.')
        .map(str::trim)
        .map(|segment| segment.strip_prefix("r#").unwrap_or(segment))
        .collect::<Vec<_>>()
        .join(".")
}
