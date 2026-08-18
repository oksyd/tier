#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
/// Runtime severity applied to a declarative validation rule.
pub enum ValidationLevel {
    /// Reject the configuration when the rule fails.
    #[default]
    Error,
    /// Record a warning and continue loading when the rule fails.
    Warning,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Additional configuration attached to a declarative validation rule.
pub struct ValidationRuleConfig {
    /// Runtime severity for the rule.
    pub level: ValidationLevel,
    /// Optional custom message shown when the rule fails.
    pub message: Option<String>,
    /// Optional machine-readable tags for downstream consumers.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
/// Numeric bound used by declarative validation rules.
pub enum ValidationNumber {
    /// A finite JSON-compatible number.
    Finite(serde_json::Number),
    /// An invalid non-finite value such as `NaN` or `inf`.
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
/// Scalar value used by declarative validation rules and conditions.
pub struct ValidationValue(pub serde_json::Value);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Declarative validation rule applied to a single configuration path.
pub enum ValidationRule {
    /// The field must not be empty.
    NonEmpty,
    /// The field must be greater than or equal to the given numeric bound.
    Min(ValidationNumber),
    /// The field must be less than or equal to the given numeric bound.
    Max(ValidationNumber),
    /// The field length must be at least the given number of units.
    MinLength(usize),
    /// The field length must be at most the given number of units.
    MaxLength(usize),
    /// The field must be an array with at least the given number of items.
    MinItems(usize),
    /// The field must be an array with at most the given number of items.
    MaxItems(usize),
    /// The field must be an object with at least the given number of properties.
    MinProperties(usize),
    /// The field must be an object with at most the given number of properties.
    MaxProperties(usize),
    /// The field must be a numeric multiple of the given factor.
    MultipleOf(ValidationNumber),
    /// The field must match the given regular expression.
    Pattern(String),
    /// The field must be an array whose items are unique.
    UniqueItems,
    /// The field must equal one of the provided scalar values.
    OneOf(Vec<ValidationValue>),
    /// The field must be a valid hostname.
    Hostname,
    /// The field must be a valid absolute URL string.
    Url,
    /// The field must be a valid email address.
    Email,
    /// The field must be a valid IP address.
    IpAddr,
    /// The field must be a valid socket address.
    SocketAddr,
    /// The field must be an absolute filesystem path.
    AbsolutePath,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Cross-field declarative validation applied to the final normalized configuration.
pub enum ValidationCheck {
    /// Requires that at least one of the given paths is configured.
    AtLeastOneOf { paths: Vec<String> },
    /// Requires that exactly one of the given paths is configured.
    ExactlyOneOf { paths: Vec<String> },
    /// Requires that no more than one of the given paths is configured.
    MutuallyExclusive { paths: Vec<String> },
    /// Requires one or more paths whenever `path` is configured.
    RequiredWith { path: String, requires: Vec<String> },
    /// Requires one or more paths whenever `path` equals `equals`.
    RequiredIf {
        /// Path whose value is inspected.
        path: String,
        /// Value that triggers the requirement.
        equals: ValidationValue,
        /// Paths that must be configured when the condition matches.
        requires: Vec<String>,
    },
}
