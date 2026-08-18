use super::super::super::{FieldMetadata, ValidationNumber, ValidationRule, ValidationValue};

impl FieldMetadata {
    /// Appends a declarative validation rule.
    #[must_use]
    pub fn validate(mut self, rule: ValidationRule) -> Self {
        self.upsert_validation(rule);
        self
    }

    /// Requires the field to be non-empty.
    #[must_use]
    pub fn non_empty(self) -> Self {
        self.validate(ValidationRule::NonEmpty)
    }

    /// Requires the field to be greater than or equal to `min`.
    #[must_use]
    pub fn min(self, min: impl Into<ValidationNumber>) -> Self {
        self.validate(ValidationRule::Min(min.into()))
    }

    /// Requires the field to be less than or equal to `max`.
    #[must_use]
    pub fn max(self, max: impl Into<ValidationNumber>) -> Self {
        self.validate(ValidationRule::Max(max.into()))
    }

    /// Requires the field length to be greater than or equal to `min`.
    #[must_use]
    pub fn min_length(self, min: usize) -> Self {
        self.validate(ValidationRule::MinLength(min))
    }

    /// Requires the field length to be less than or equal to `max`.
    #[must_use]
    pub fn max_length(self, max: usize) -> Self {
        self.validate(ValidationRule::MaxLength(max))
    }

    /// Requires the field to be an array with at least `min` items.
    #[must_use]
    pub fn min_items(self, min: usize) -> Self {
        self.validate(ValidationRule::MinItems(min))
    }

    /// Requires the field to be an array with at most `max` items.
    #[must_use]
    pub fn max_items(self, max: usize) -> Self {
        self.validate(ValidationRule::MaxItems(max))
    }

    /// Requires the field to be an object with at least `min` properties.
    #[must_use]
    pub fn min_properties(self, min: usize) -> Self {
        self.validate(ValidationRule::MinProperties(min))
    }

    /// Requires the field to be an object with at most `max` properties.
    #[must_use]
    pub fn max_properties(self, max: usize) -> Self {
        self.validate(ValidationRule::MaxProperties(max))
    }

    /// Requires the field to be an exact multiple of `factor`.
    #[must_use]
    pub fn multiple_of(self, factor: impl Into<ValidationNumber>) -> Self {
        self.validate(ValidationRule::MultipleOf(factor.into()))
    }

    /// Requires the field to match a regular expression.
    #[must_use]
    pub fn pattern(self, pattern: impl Into<String>) -> Self {
        self.validate(ValidationRule::Pattern(pattern.into()))
    }

    /// Requires the field to be an array with unique items.
    #[must_use]
    pub fn unique_items(self) -> Self {
        self.validate(ValidationRule::UniqueItems)
    }

    /// Requires the field to match one of the provided scalar values.
    #[must_use]
    pub fn one_of<I, V>(self, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<ValidationValue>,
    {
        self.validate(ValidationRule::OneOf(
            values.into_iter().map(Into::into).collect(),
        ))
    }

    /// Requires the field to be a valid hostname.
    #[must_use]
    pub fn hostname(self) -> Self {
        self.validate(ValidationRule::Hostname)
    }

    /// Requires the field to be a valid absolute URL string.
    #[must_use]
    pub fn url(self) -> Self {
        self.validate(ValidationRule::Url)
    }

    /// Requires the field to be a valid email address.
    #[must_use]
    pub fn email(self) -> Self {
        self.validate(ValidationRule::Email)
    }

    /// Requires the field to be a valid IP address.
    #[must_use]
    pub fn ip_addr(self) -> Self {
        self.validate(ValidationRule::IpAddr)
    }

    /// Requires the field to be a valid socket address.
    #[must_use]
    pub fn socket_addr(self) -> Self {
        self.validate(ValidationRule::SocketAddr)
    }

    /// Requires the field to be an absolute filesystem path.
    #[must_use]
    pub fn absolute_path(self) -> Self {
        self.validate(ValidationRule::AbsolutePath)
    }

    pub(in crate::metadata) fn upsert_validation(&mut self, rule: ValidationRule) {
        if let Some(existing) = self
            .validations
            .iter_mut()
            .find(|existing| existing.code() == rule.code())
        {
            *existing = rule;
        } else {
            self.validations.push(rule);
        }
    }
}
