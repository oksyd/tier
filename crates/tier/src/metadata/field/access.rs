use std::collections::{BTreeMap, BTreeSet};

use crate::loader::SourceKind;

use super::super::{
    EnvDecoder, FieldMetadata, MergeStrategy, ValidationRule, ValidationRuleConfig,
};

impl FieldMetadata {
    /// Returns the normalized dot-delimited configuration path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the normalized alternate paths accepted during deserialization.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// Returns whether values at this path are sensitive.
    #[must_use]
    pub fn is_secret(&self) -> bool {
        self.secret
    }

    /// Returns the exact environment variable name configured for this path.
    #[must_use]
    pub fn env_name(&self) -> Option<&str> {
        self.env.as_deref()
    }

    /// Returns the decoder applied to environment values for this path.
    #[must_use]
    pub fn decoder(&self) -> Option<EnvDecoder> {
        self.env_decode
    }

    /// Returns the human-readable field documentation.
    #[must_use]
    pub fn documentation(&self) -> Option<&str> {
        self.doc.as_deref()
    }

    /// Returns the example value used by generated documentation.
    #[must_use]
    pub fn example_value(&self) -> Option<&str> {
        self.example.as_deref()
    }

    /// Returns the field's deprecation note.
    #[must_use]
    pub fn deprecation_note(&self) -> Option<&str> {
        self.deprecated.as_deref()
    }

    /// Returns whether omission is accepted through `serde(default)`.
    #[must_use]
    pub fn has_default(&self) -> bool {
        self.has_default
    }

    /// Returns the effective field-level merge strategy.
    #[must_use]
    pub fn effective_merge_strategy(&self) -> MergeStrategy {
        self.merge
    }

    /// Returns whether the merge strategy was explicitly declared.
    #[must_use]
    pub fn has_explicit_merge_strategy(&self) -> bool {
        self.merge_explicit
    }

    /// Returns the source kinds allowed to override this field, when restricted.
    #[must_use]
    pub fn allowed_sources(&self) -> Option<&BTreeSet<SourceKind>> {
        self.allowed_sources.as_ref()
    }

    /// Returns the source kinds denied from overriding this field, when restricted.
    #[must_use]
    pub fn denied_sources(&self) -> Option<&BTreeSet<SourceKind>> {
        self.denied_sources.as_ref()
    }

    /// Returns the declarative validation rules for this field.
    #[must_use]
    pub fn validations(&self) -> &[ValidationRule] {
        &self.validations
    }

    /// Returns per-rule configuration keyed by validation rule code.
    #[must_use]
    pub fn validation_configs(&self) -> &BTreeMap<String, ValidationRuleConfig> {
        &self.validation_configs
    }
}
