mod access;
mod builder;
mod export;
mod merge;
mod source_policy;
mod state;
mod validation;

#[cfg(feature = "schema")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FieldValidationExport {
    pub(crate) levels: std::collections::BTreeMap<String, super::ValidationLevel>,
    pub(crate) messages: std::collections::BTreeMap<String, String>,
    pub(crate) tags: std::collections::BTreeMap<String, Vec<String>>,
}
