use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use thiserror::Error;

use crate::loader::{FileFormat, SourceTrace};

mod format;
mod model;
mod validation;

use self::format::{
    deserialize_source_suffix, format_location, format_missing_paths, format_path_location,
    format_source_policy, format_unknown_fields,
};
pub use self::model::{LineColumn, UnknownField};
pub use self::validation::{
    PathProvenance, ValidationError, ValidationErrors, ValidationFailure, ValidationFailures,
    ValidatorKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Part of an environment variable that could not be decoded as Unicode.
pub enum EnvironmentVariableComponent {
    /// Environment variable name.
    Name,
    /// Environment variable value.
    Value,
}

impl Display for EnvironmentVariableComponent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Name => "name",
            Self::Value => "value",
        })
    }
}

#[derive(Debug, Error)]
/// Errors returned while building, loading, validating, or inspecting configuration.
pub enum ConfigError {
    /// The root serialized value was not a JSON-like object.
    #[error("configuration root must serialize to a map-like object, got {actual}")]
    RootMustBeObject {
        /// Human-readable kind of the unexpected root value.
        actual: &'static str,
    },

    /// Serializing configuration state into an intermediate value failed.
    #[error("failed to serialize configuration state: {source}")]
    Serialize {
        /// Serialization error from the intermediate serde representation.
        #[from]
        source: serde_json::Error,
    },

    /// Starting or running a filesystem watcher failed.
    #[error("failed to watch configuration files: {message}")]
    Watch {
        /// Human-readable watcher failure details.
        message: String,
    },

    /// A relevant process environment variable could not be represented as UTF-8.
    #[error(
        "environment variable `{name}` has a non-Unicode {component}",
        name = name.to_string_lossy()
    )]
    NonUnicodeEnvironment {
        /// Variable name, rendered lossily only for diagnostics.
        name: OsString,
        /// Variable component that could not be decoded.
        component: EnvironmentVariableComponent,
    },

    /// A process argument could not be represented as UTF-8.
    #[error("process argument at index {index} is not valid Unicode")]
    NonUnicodeArgument {
        /// Zero-based argument index.
        index: usize,
    },

    /// A required configuration file was not found.
    #[error("required configuration file not found: {}", path.display())]
    MissingFile {
        /// Missing file path.
        path: PathBuf,
    },

    /// None of the required candidate files were found.
    #[error("none of the required configuration files were found:\n{paths}", paths = format_missing_paths(paths))]
    MissingFiles {
        /// Candidate paths that were checked.
        paths: Vec<PathBuf>,
    },

    /// Reading a configuration file failed.
    #[error("failed to read configuration file {}: {source}", path.display())]
    ReadFile {
        /// File path being read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Parsing a configuration file failed.
    #[error("failed to parse {format} configuration file {}{location}: {message}", path.display(), location = format_location(*location))]
    ParseFile {
        /// File path being parsed.
        path: PathBuf,
        /// Detected or configured file format.
        format: FileFormat,
        /// Optional line and column information for the parse error.
        location: Option<LineColumn>,
        /// Human-readable parse failure.
        message: String,
    },

    /// An environment variable could not be converted into configuration data.
    #[error("invalid environment variable {name} for path {path}: {message}")]
    InvalidEnv {
        /// Environment variable name.
        name: String,
        /// Derived configuration path.
        path: String,
        /// Human-readable conversion failure.
        message: String,
    },

    /// A CLI override argument was invalid.
    #[error("invalid CLI argument {arg}: {message}")]
    InvalidArg {
        /// Raw CLI fragment.
        arg: String,
        /// Human-readable validation failure.
        message: String,
    },

    /// A typed sparse patch could not be converted into a configuration layer.
    #[error("invalid patch {name} for path {path}: {message}")]
    InvalidPatch {
        /// Human-readable patch source name.
        name: String,
        /// Target configuration path.
        path: String,
        /// Human-readable validation failure.
        message: String,
    },

    /// Multiple input paths resolved to the same canonical path.
    #[error(
        "configuration paths `{first_path}` and `{second_path}` both resolve to `{canonical_path}`"
    )]
    PathConflict {
        /// First input path that mapped to the canonical path.
        first_path: String,
        /// Second conflicting input path.
        second_path: String,
        /// Canonical path both inputs resolved to.
        canonical_path: String,
    },

    /// A source attempted to write to a field that restricts allowed source kinds.
    #[error(
        "source {trace} is not allowed to set `{path}`; {policy}",
        policy = format_source_policy(allowed_sources, denied_sources)
    )]
    SourcePolicyViolation {
        /// Concrete path rejected by the source policy.
        path: String,
        /// Actual source attempting to set the path.
        trace: SourceTrace,
        /// Allowed source kinds for the path.
        allowed_sources: Box<[crate::loader::SourceKind]>,
        /// Explicitly denied source kinds for the path.
        denied_sources: Box<[crate::loader::SourceKind]>,
    },

    /// The loaded configuration declares a version newer than this binary supports.
    #[error(
        "configuration version at `{path}` is {found}, but this binary only supports up to {supported}"
    )]
    UnsupportedConfigVersion {
        /// Path that stores the configuration version.
        path: String,
        /// Version found in the loaded configuration.
        found: u32,
        /// Highest version understood by this binary.
        supported: u32,
    },

    /// The loaded configuration version field could not be parsed as an unsigned integer.
    #[error("configuration version at `{path}` must be an unsigned integer: {message}")]
    InvalidConfigVersion {
        /// Path that stores the configuration version.
        path: String,
        /// Human-readable validation failure.
        message: String,
    },

    /// Both sides of a rename migration were explicitly configured.
    #[error(
        "cannot rename configuration path `{from_path}` to `{to_path}` because the target already has an explicit value"
    )]
    MigrationConflict {
        /// Legacy path containing the value to migrate.
        from_path: String,
        /// Target path that was also explicitly configured.
        to_path: String,
        /// Source that provided the target value, when known.
        provenance: Option<SourceTrace>,
    },

    /// A serialized object key could not be represented in tier's dot-delimited path model.
    #[error(
        "configuration object key `{key}` under {location} cannot be represented in tier paths: {message}",
        location = format_path_location(path)
    )]
    InvalidPathKey {
        /// Parent path containing the unsupported key.
        path: String,
        /// Unsupported object key segment.
        key: String,
        /// Human-readable validation failure.
        message: String,
    },

    /// Metadata declared the same alias or environment variable more than once.
    #[error("metadata {kind} `{name}` is assigned to both `{first_path}` and `{second_path}`")]
    MetadataConflict {
        /// Human-readable conflict category such as `alias` or `environment variable`.
        kind: &'static str,
        /// Conflicting alias or environment variable name.
        name: String,
        /// First path using the name.
        first_path: String,
        /// Second path using the name.
        second_path: String,
    },

    /// Metadata declared an unsupported or invalid field configuration.
    #[error("invalid metadata for `{path}`: {message}")]
    MetadataInvalid {
        /// Metadata path that triggered the validation failure.
        path: String,
        /// Human-readable validation failure.
        message: String,
    },

    /// A CLI flag requiring a value was missing one.
    #[error("missing value for CLI flag {flag}")]
    MissingArgValue {
        /// Flag name missing a required value.
        flag: String,
    },

    /// A file path template referenced `{profile}` without a profile being set.
    #[error("path template {} contains {{profile}} but no profile was set", path.display())]
    MissingProfile {
        /// Path template containing `{profile}`.
        path: PathBuf,
    },

    /// Deserializing the merged intermediate value into the target type failed.
    #[error(
        "failed to deserialize merged configuration at {path}: {message}{source_suffix}",
        source_suffix = deserialize_source_suffix(provenance)
    )]
    Deserialize {
        /// Configuration path reported by serde.
        path: String,
        /// Most recent source that contributed the failing value, when known.
        provenance: Option<SourceTrace>,
        /// Human-readable deserialization failure.
        message: String,
    },

    /// The requested explain path did not exist in the final report.
    #[error(
        "cannot explain configuration path `{path}` because it does not exist in the final report"
    )]
    ExplainPathNotFound {
        /// Requested explain path.
        path: String,
    },

    /// Unknown configuration paths were found and the active policy rejected them.
    #[error("unknown configuration fields:\n{fields}", fields = format_unknown_fields(fields))]
    UnknownFields {
        /// Unknown paths discovered during loading.
        fields: Vec<UnknownField>,
    },

    /// A normalizer hook failed.
    #[error("normalizer {name} failed: {message}")]
    Normalize {
        /// Normalizer name.
        name: String,
        /// Human-readable failure.
        message: String,
    },

    /// One or more validation stages failed.
    #[error("configuration validation failed:\n{failures}")]
    Validation {
        /// Ordered failures returned by declared and custom validators.
        failures: ValidationFailures,
    },
}

impl ConfigError {
    /// Renders the error in a CLI-friendly form for terminal output.
    #[must_use]
    pub fn cli_message(&self) -> String {
        match self {
            Self::UnknownFields { fields } => {
                format!(
                    "Unknown configuration fields:\n{}",
                    format_unknown_fields(fields)
                )
            }
            Self::Validation { failures } => {
                format!("Configuration validation failed:\n{failures}")
            }
            Self::ExplainPathNotFound { path } => {
                format!("Configuration path `{path}` was not found in the final report")
            }
            _ => format!("Configuration error: {self}"),
        }
    }
}
