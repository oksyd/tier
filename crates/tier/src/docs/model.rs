use std::collections::BTreeMap;

use crate::{MergeStrategy, SourceKind, ValidationLevel, ValidationRule};

/// Stable version tag for machine-readable environment documentation payloads.
pub const ENV_DOCS_FORMAT_VERSION: u32 = 3;

/// A single schema-derived environment variable documentation row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvDocEntry {
    /// Dot-delimited configuration path.
    pub path: String,
    /// Environment variable name corresponding to the path.
    ///
    /// Collection segments are rendered as placeholder tokens such as `{item}`.
    /// Replace them with a concrete index or key when setting an actual
    /// environment variable.
    pub env: String,
    /// Human-readable schema type summary.
    pub ty: String,
    /// Whether the field is required by the schema.
    pub required: bool,
    /// Whether the field should be treated as sensitive.
    pub secret: bool,
    /// Optional field description pulled from the schema.
    pub description: Option<String>,
    /// Optional example value provided by metadata.
    pub example: Option<String>,
    /// Optional deprecation note provided by metadata.
    pub deprecated: Option<String>,
    /// Alternate deserialize aliases accepted for this field.
    pub aliases: Vec<String>,
    /// Whether the field can be omitted because deserialization supplies a default.
    pub has_default: bool,
    /// Merge policy applied when multiple layers target this field.
    pub merge: MergeStrategy,
    /// Source kinds allowed to override this field.
    ///
    /// An empty list means the field does not restrict its allowed sources.
    pub allowed_sources: Vec<SourceKind>,
    /// Source kinds explicitly denied from overriding this field.
    pub denied_sources: Vec<SourceKind>,
    /// Declarative validation rules applied to this field.
    pub validations: Vec<ValidationRule>,
    /// Per-rule validation levels keyed by rule code.
    pub validation_levels: BTreeMap<String, ValidationLevel>,
    /// Per-rule custom messages keyed by rule code.
    pub validation_messages: BTreeMap<String, String>,
    /// Per-rule machine-readable tags keyed by rule code.
    pub validation_tags: BTreeMap<String, Vec<String>>,
}

/// Versioned machine-readable environment documentation payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvDocsReport {
    /// Stable schema version for external consumers.
    pub format_version: u32,
    /// Rendered environment variable documentation entries.
    pub entries: Vec<EnvDocEntry>,
}

/// Options controlling schema-derived environment variable documentation.
///
/// Use `EnvDocOptions` to keep generated env docs aligned with the same prefix
/// and separator conventions you use at runtime.
///
/// # Examples
///
/// ```
/// use tier::EnvDocOptions;
///
/// let env = EnvDocOptions::prefixed("APP").env_name("server.port");
/// assert_eq!(env, "APP__SERVER__PORT");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvDocOptions {
    prefix: Option<String>,
    separator: String,
    uppercase: bool,
}

impl Default for EnvDocOptions {
    fn default() -> Self {
        Self {
            prefix: None,
            separator: "__".to_owned(),
            uppercase: true,
        }
    }
}

impl EnvDocOptions {
    /// Creates options with no prefix, `__` separator, and uppercase names.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates options with a fixed environment variable prefix.
    #[must_use]
    pub fn prefixed(prefix: impl Into<String>) -> Self {
        Self::default().prefix(prefix)
    }

    /// Sets the environment variable prefix.
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the separator placed between path segments.
    #[must_use]
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        let separator = separator.into();
        if !separator.is_empty() {
            self.separator = separator;
        }
        self
    }

    /// Preserves field case instead of uppercasing environment names.
    #[must_use]
    pub fn preserve_case(mut self) -> Self {
        self.uppercase = false;
        self
    }

    /// Converts a dot-delimited configuration path into an environment variable name.
    ///
    /// Collection segments are rendered as placeholder tokens so generated docs
    /// describe a template rather than the internal wildcard syntax.
    #[must_use]
    pub fn env_name(&self, path: &str) -> String {
        crate::env_name::path_to_env_name(
            path,
            self.prefix.as_deref(),
            &self.separator,
            self.uppercase,
        )
    }
}
