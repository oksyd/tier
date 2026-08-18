use std::fmt::{self, Display, Formatter};

use serde_json::Value;

use crate::loader::SourceTrace;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Source that most recently contributed a path involved in a validation issue.
pub struct PathProvenance {
    /// Canonical configuration path.
    pub path: String,
    /// Most recent source for the path.
    pub source: SourceTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A single validation failure returned by a validator hook.
pub struct ValidationError {
    /// Dot-delimited configuration path associated with the failure.
    pub path: String,
    /// Additional paths related to the failure, used for cross-field validations.
    pub related_paths: Vec<String>,
    /// Human-readable failure message.
    pub message: String,
    /// Optional rule identifier for machine-readable consumers.
    pub rule: Option<String>,
    /// Optional expected value associated with the failed rule.
    pub expected: Option<Value>,
    /// Optional actual value observed during validation.
    pub actual: Option<Value>,
    /// Optional machine-readable tags for downstream consumers.
    pub tags: Vec<String>,
    /// Most recent sources for the primary and related paths.
    pub provenance: Vec<PathProvenance>,
}

impl ValidationError {
    /// Creates a new validation error.
    ///
    /// Keep `message` independent of runtime values. The loader redacts the
    /// structured `actual` and `expected` payloads for secret paths, but it
    /// cannot safely parse and rewrite arbitrary human-readable text.
    #[must_use]
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            related_paths: Vec::new(),
            message: message.into(),
            rule: None,
            expected: None,
            actual: None,
            tags: Vec::new(),
            provenance: Vec::new(),
        }
    }

    /// Attaches a machine-readable rule identifier.
    #[must_use]
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Attaches related paths for cross-field validation failures.
    #[must_use]
    pub fn with_related_paths<I, S>(mut self, related_paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.related_paths = related_paths.into_iter().map(Into::into).collect();
        self
    }

    /// Attaches the expected value for the failed rule.
    #[must_use]
    pub fn with_expected(mut self, expected: Value) -> Self {
        self.expected = Some(expected);
        self
    }

    /// Attaches the actual value observed during validation.
    #[must_use]
    pub fn with_actual(mut self, actual: Value) -> Self {
        self.actual = Some(actual);
        self
    }

    /// Attaches machine-readable tags for downstream consumers.
    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Attaches path-level source information.
    #[must_use]
    pub fn with_provenance<I>(mut self, provenance: I) -> Self
    where
        I: IntoIterator<Item = PathProvenance>,
    {
        self.provenance = provenance.into_iter().collect();
        self
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)?;
        } else {
            write!(f, "{}: {}", self.path, self.message)?;
        }

        match self.provenance.as_slice() {
            [] => Ok(()),
            [entry] => write!(f, " (from {})", entry.source),
            entries => {
                f.write_str(" (sources: ")?;
                for (index, entry) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{} from {}", entry.path, entry.source)?;
                }
                f.write_str(")")
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// A collection of validation failures returned by a validator hook.
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

impl ValidationErrors {
    /// Creates an empty validation error collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a collection containing a single validation error.
    #[must_use]
    pub fn from_error(error: ValidationError) -> Self {
        Self {
            errors: vec![error],
        }
    }

    /// Creates a collection containing a single message-based validation error.
    #[must_use]
    pub fn from_message(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::from_error(ValidationError::new(path, message))
    }

    /// Appends a validation error.
    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Appends multiple validation errors.
    pub fn extend<I>(&mut self, errors: I)
    where
        I: IntoIterator<Item = ValidationError>,
    {
        self.errors.extend(errors);
    }

    /// Returns `true` when the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of validation errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Consumes the collection into a vector.
    pub fn into_vec(self) -> Vec<ValidationError> {
        self.errors
    }

    /// Returns an iterator over validation errors.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut ValidationError> {
        self.errors.iter_mut()
    }
}

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl Display for ValidationErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "- {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
/// Identifies the validation stage that produced a failure.
pub enum ValidatorKind {
    /// Metadata-driven field and cross-field validation.
    Declared,
    /// A named application validator hook.
    Custom(String),
}

impl Display for ValidatorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declared => formatter.write_str("declared validation"),
            Self::Custom(name) => write!(formatter, "validator {name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// All issues returned by one validation stage or custom validator.
pub struct ValidationFailure {
    /// Validator that produced the issues.
    pub validator: ValidatorKind,
    /// Issues produced by the validator.
    pub errors: ValidationErrors,
}

impl ValidationFailure {
    /// Creates a failure for metadata-driven validation.
    #[must_use]
    pub fn declared(errors: ValidationErrors) -> Self {
        Self {
            validator: ValidatorKind::Declared,
            errors,
        }
    }

    /// Creates a failure for a named custom validator.
    #[must_use]
    pub fn custom(name: impl Into<String>, errors: ValidationErrors) -> Self {
        Self {
            validator: ValidatorKind::Custom(name.into()),
            errors,
        }
    }
}

impl Display for ValidationFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} failed:\n{}", self.validator, self.errors)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Ordered validation failures collected across all validation stages.
pub struct ValidationFailures {
    failures: Vec<ValidationFailure>,
}

impl ValidationFailures {
    /// Creates an empty validation failure collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one validator failure.
    pub fn push(&mut self, failure: ValidationFailure) {
        self.failures.push(failure);
    }

    /// Returns whether no validator failed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.failures.is_empty()
    }

    /// Returns the number of failed validators or validation stages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.failures.len()
    }

    /// Iterates over failures in validation order.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationFailure> {
        self.failures.iter()
    }

    /// Consumes the collection into its ordered failures.
    pub fn into_vec(self) -> Vec<ValidationFailure> {
        self.failures
    }
}

impl IntoIterator for ValidationFailures {
    type Item = ValidationFailure;
    type IntoIter = std::vec::IntoIter<ValidationFailure>;

    fn into_iter(self) -> Self::IntoIter {
        self.failures.into_iter()
    }
}

impl Display for ValidationFailures {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.failures.iter().enumerate() {
            if index > 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{failure}")?;
        }
        Ok(())
    }
}
