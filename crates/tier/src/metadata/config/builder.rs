use super::super::ValidationCheckSpec;
use super::super::{ConfigMetadata, FieldMetadata, ValidationCheck, ValidationValue};

impl ConfigMetadata {
    /// Creates an empty metadata set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates metadata from a list of field entries.
    #[must_use]
    pub fn from_fields<I>(fields: I) -> Self
    where
        I: IntoIterator<Item = FieldMetadata>,
    {
        let mut metadata = Self::default();
        metadata.extend_fields(fields);
        metadata
    }

    /// Adds a field metadata entry and merges duplicates by path.
    pub fn push(&mut self, field: FieldMetadata) {
        self.fields.push(field);
        self.normalize();
    }

    /// Extends the metadata with additional field entries.
    pub fn extend_fields<I>(&mut self, fields: I)
    where
        I: IntoIterator<Item = FieldMetadata>,
    {
        self.fields.extend(fields);
        self.normalize();
    }

    /// Extends the metadata with another metadata set.
    pub fn extend(&mut self, other: Self) {
        self.fields.extend(other.fields);
        self.check_specs.extend(other.check_specs);
        self.pending_checks.extend(other.pending_checks);
        self.normalize();
    }

    /// Adds a cross-field validation check.
    pub fn push_check(&mut self, check: ValidationCheck) {
        self.pending_checks.push(check);
        self.normalize();
    }

    /// Extends the metadata with additional cross-field validation checks.
    pub fn extend_checks<I>(&mut self, checks: I)
    where
        I: IntoIterator<Item = ValidationCheck>,
    {
        self.pending_checks.extend(checks);
        self.normalize();
    }

    pub(crate) fn extend_check_specs<I>(&mut self, checks: I)
    where
        I: IntoIterator<Item = ValidationCheckSpec>,
    {
        self.check_specs.extend(checks);
        self.normalize();
    }

    /// Adds a cross-field validation check in builder style.
    #[must_use]
    pub fn check(mut self, check: ValidationCheck) -> Self {
        self.push_check(check);
        self
    }

    /// Requires that at least one of the given paths is configured.
    #[must_use]
    pub fn at_least_one_of<I, S>(self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.check(ValidationCheck::AtLeastOneOf {
            paths: paths.into_iter().map(Into::into).collect(),
        })
    }

    /// Requires that exactly one of the given paths is configured.
    #[must_use]
    pub fn exactly_one_of<I, S>(self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.check(ValidationCheck::ExactlyOneOf {
            paths: paths.into_iter().map(Into::into).collect(),
        })
    }

    /// Requires that at most one of the given paths is configured.
    #[must_use]
    pub fn mutually_exclusive<I, S>(self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.check(ValidationCheck::MutuallyExclusive {
            paths: paths.into_iter().map(Into::into).collect(),
        })
    }

    /// Requires one or more paths whenever `path` is configured.
    #[must_use]
    pub fn required_with<I, S>(self, path: impl Into<String>, requires: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.check(ValidationCheck::RequiredWith {
            path: path.into(),
            requires: requires.into_iter().map(Into::into).collect(),
        })
    }

    /// Requires one or more paths whenever `path` equals `equals`.
    #[must_use]
    pub fn required_if<I, S, V>(self, path: impl Into<String>, equals: V, requires: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        V: Into<ValidationValue>,
    {
        self.check(ValidationCheck::RequiredIf {
            path: path.into(),
            equals: equals.into(),
            requires: requires.into_iter().map(Into::into).collect(),
        })
    }
}
