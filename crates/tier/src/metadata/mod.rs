use std::collections::{BTreeMap, BTreeSet};

use crate::loader::SourceKind;

mod config;
mod display;
mod env;
mod field;
mod impls;
mod merge;
mod paths;
mod prefix;
mod validation;

pub use self::env::EnvDecoder;
pub use self::merge::MergeStrategy;
pub use self::prefix::prefixed_metadata;
pub use self::validation::{
    ValidationCheck, ValidationLevel, ValidationNumber, ValidationRule, ValidationRuleConfig,
    ValidationValue,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Structured metadata describing configuration fields.
///
/// `ConfigMetadata` is the manual metadata API behind `tier`'s higher-level
/// derive support. It can describe:
///
/// - field-level behavior such as env names, aliases, secret paths, examples,
///   merge policies, and declared validation rules
/// - cross-field validation checks such as mutually exclusive or required-if
///   relationships
///
/// # Examples
///
/// ```
/// use tier::{ConfigMetadata, FieldMetadata};
///
/// let metadata = ConfigMetadata::from_fields([
///     FieldMetadata::new("db.url").env("DATABASE_URL"),
///     FieldMetadata::new("db.password").secret(),
/// ])
/// .required_with("tls.enabled", ["tls.cert", "tls.key"]);
///
/// assert_eq!(
///     metadata
///         .env_overrides()
///         .expect("valid metadata")
///         .get("DATABASE_URL")
///         .map(String::as_str),
///     Some("db.url")
/// );
/// assert_eq!(metadata.secret_paths(), vec!["db.password".to_owned()]);
/// assert_eq!(metadata.checks().len(), 1);
/// ```
pub struct ConfigMetadata {
    fields: Vec<FieldMetadata>,
    pending_checks: Vec<ValidationCheck>,
    checks: Vec<ValidationCheck>,
    check_specs: Vec<ValidationCheckSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MetadataMatchScore {
    segment_count: usize,
    specificity: usize,
    positional_specificity: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Metadata for a single configuration path.
pub struct FieldMetadata {
    pub(crate) path: String,
    pub(crate) path_explicit_array_segments: BTreeSet<usize>,
    pub(crate) aliases: Vec<String>,
    pub(crate) alias_explicit_array_segments: BTreeMap<String, BTreeSet<usize>>,
    pub(crate) secret: bool,
    pub(crate) env: Option<String>,
    pub(crate) env_decode: Option<EnvDecoder>,
    pub(crate) doc: Option<String>,
    pub(crate) example: Option<String>,
    pub(crate) deprecated: Option<String>,
    pub(crate) has_default: bool,
    pub(crate) merge: MergeStrategy,
    pub(crate) merge_explicit: bool,
    pub(crate) allowed_sources: Option<BTreeSet<SourceKind>>,
    pub(crate) denied_sources: Option<BTreeSet<SourceKind>>,
    pub(crate) validations: Vec<ValidationRule>,
    pub(crate) validation_configs: BTreeMap<String, ValidationRuleConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EffectiveSourcePolicy {
    pub(crate) allowed_sources: Option<BTreeSet<SourceKind>>,
    pub(crate) denied_sources: Option<BTreeSet<SourceKind>>,
}

pub(crate) struct EffectiveValidation {
    pub(crate) field: FieldMetadata,
    pub(crate) rule: ValidationRule,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MetadataPathSpec {
    pub(crate) path: String,
    pub(crate) explicit_array_segments: BTreeSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AliasOverrideSpec {
    pub(crate) alias: String,
    pub(crate) alias_explicit_array_segments: BTreeSet<usize>,
    pub(crate) canonical: String,
    pub(crate) canonical_explicit_array_segments: BTreeSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvOverrideSpec {
    pub(crate) env: String,
    pub(crate) path: String,
    pub(crate) explicit_array_segments: BTreeSet<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ValidationCheckSpec {
    AtLeastOneOf {
        paths: Vec<MetadataPathSpec>,
    },
    ExactlyOneOf {
        paths: Vec<MetadataPathSpec>,
    },
    MutuallyExclusive {
        paths: Vec<MetadataPathSpec>,
    },
    RequiredWith {
        path: MetadataPathSpec,
        requires: Vec<MetadataPathSpec>,
    },
    RequiredIf {
        path: MetadataPathSpec,
        equals: ValidationValue,
        requires: Vec<MetadataPathSpec>,
    },
}

/// Metadata produced for a configuration type.
pub trait TierMetadata {
    /// Returns metadata for the configuration type.
    #[must_use]
    fn metadata() -> ConfigMetadata {
        ConfigMetadata::default()
    }

    /// Returns configuration paths that should be treated as secrets.
    #[must_use]
    fn secret_paths() -> Vec<String> {
        Self::metadata().secret_paths()
    }
}
