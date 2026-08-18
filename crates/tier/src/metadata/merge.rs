#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Strategy applied when multiple layers write to the same configuration path.
pub enum MergeStrategy {
    /// Recursively merge objects and replace non-object values.
    #[default]
    Merge,
    /// Replace the current value at this path with the overlay value.
    Replace,
    /// Append array overlays while still recursively merging nested objects.
    Append,
}
