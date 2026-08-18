/// Hidden helper used by the `TierPatch` derive macro.
#[doc(hidden)]
#[must_use]
pub fn join_patch_prefix(prefix: &str, path: impl AsRef<str>) -> String {
    let path = path.as_ref();
    match (prefix.is_empty(), path.is_empty()) {
        (true, true) => String::new(),
        (true, false) => path.to_owned(),
        (false, true) => prefix.to_owned(),
        (false, false) => format!("{prefix}.{path}"),
    }
}
