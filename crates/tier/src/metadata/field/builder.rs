use std::collections::BTreeMap;

use crate::loader::SourceKind;

use super::super::paths::normalize_metadata_path_with_explicit_arrays;
use super::super::{EnvDecoder, FieldMetadata, MergeStrategy};

impl FieldMetadata {
    /// Creates metadata for a single configuration path.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        let (path, path_explicit_array_segments) =
            normalize_metadata_path_with_explicit_arrays(&path.into());
        Self {
            path,
            path_explicit_array_segments,
            aliases: Vec::new(),
            alias_explicit_array_segments: BTreeMap::new(),
            secret: false,
            env: None,
            env_decode: None,
            doc: None,
            example: None,
            deprecated: None,
            has_default: false,
            merge: MergeStrategy::Merge,
            merge_explicit: false,
            allowed_sources: None,
            denied_sources: None,
            validations: Vec::new(),
            validation_configs: BTreeMap::new(),
        }
    }

    /// Adds an alternate serde path for this field.
    #[must_use]
    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        let (alias, explicit_array_segments) =
            normalize_metadata_path_with_explicit_arrays(&alias.into());
        if !explicit_array_segments.is_empty() {
            self.alias_explicit_array_segments
                .insert(alias.clone(), explicit_array_segments);
        }
        self.aliases.push(alias);
        self
    }

    /// Marks this path as sensitive.
    #[must_use]
    pub fn secret(mut self) -> Self {
        self.secret = true;
        self
    }

    /// Overrides the environment variable name for this path.
    #[must_use]
    pub fn env(mut self, env: impl Into<String>) -> Self {
        self.env = Some(env.into());
        self
    }

    /// Decodes environment variables for this path with a built-in decoder.
    ///
    /// This can be used together with [`ConfigMetadata`](crate::ConfigMetadata) when metadata is
    /// built manually instead of derived.
    #[must_use]
    pub fn env_decoder(mut self, decoder: EnvDecoder) -> Self {
        self.env_decode = Some(decoder);
        self
    }

    /// Adds human-readable field documentation.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
    }

    /// Adds an example value used by generated docs.
    #[must_use]
    pub fn example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Marks the field as deprecated with an optional note.
    #[must_use]
    pub fn deprecated(mut self, note: impl Into<String>) -> Self {
        self.deprecated = Some(note.into());
        self
    }

    /// Marks the field as accepting omission via `serde(default)`.
    #[must_use]
    pub fn defaulted(mut self) -> Self {
        self.has_default = true;
        self
    }

    /// Sets the field-level merge strategy.
    #[must_use]
    pub fn merge_strategy(mut self, merge: MergeStrategy) -> Self {
        self.merge = merge;
        self.merge_explicit = true;
        self
    }

    /// Restricts the field to a specific set of source kinds.
    ///
    /// This is useful for fields that should only be loaded from selected
    /// layers, such as requiring secrets to come from environment variables or
    /// disallowing file-based overrides for a path.
    #[must_use]
    pub fn allow_sources<I>(mut self, sources: I) -> Self
    where
        I: IntoIterator<Item = SourceKind>,
    {
        self.allowed_sources = Some(sources.into_iter().collect());
        self
    }

    /// Explicitly denies a set of source kinds from overriding this field.
    #[must_use]
    pub fn deny_sources<I>(mut self, sources: I) -> Self
    where
        I: IntoIterator<Item = SourceKind>,
    {
        self.denied_sources = Some(sources.into_iter().collect());
        self
    }
}
