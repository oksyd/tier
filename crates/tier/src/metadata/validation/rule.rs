use std::fmt::{self, Display, Formatter};

use super::super::ValidationRule;

impl ValidationRule {
    /// Returns a stable machine-readable rule identifier.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NonEmpty => "non_empty",
            Self::Min(_) => "min",
            Self::Max(_) => "max",
            Self::MinLength(_) => "min_length",
            Self::MaxLength(_) => "max_length",
            Self::MinItems(_) => "min_items",
            Self::MaxItems(_) => "max_items",
            Self::MinProperties(_) => "min_properties",
            Self::MaxProperties(_) => "max_properties",
            Self::MultipleOf(_) => "multiple_of",
            Self::Pattern(_) => "pattern",
            Self::UniqueItems => "unique_items",
            Self::OneOf(_) => "one_of",
            Self::Hostname => "hostname",
            Self::Url => "url",
            Self::Email => "email",
            Self::IpAddr => "ip_addr",
            Self::SocketAddr => "socket_addr",
            Self::AbsolutePath => "absolute_path",
        }
    }
}

impl Display for ValidationRule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonEmpty => write!(f, "non_empty"),
            Self::Min(value) => write!(f, "min={value}"),
            Self::Max(value) => write!(f, "max={value}"),
            Self::MinLength(value) => write!(f, "min_length={value}"),
            Self::MaxLength(value) => write!(f, "max_length={value}"),
            Self::MinItems(value) => write!(f, "min_items={value}"),
            Self::MaxItems(value) => write!(f, "max_items={value}"),
            Self::MinProperties(value) => write!(f, "min_properties={value}"),
            Self::MaxProperties(value) => write!(f, "max_properties={value}"),
            Self::MultipleOf(value) => write!(f, "multiple_of={value}"),
            Self::Pattern(value) => write!(f, "pattern={value:?}"),
            Self::UniqueItems => write!(f, "unique_items"),
            Self::OneOf(values) => write!(
                f,
                "one_of=[{}]",
                values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Hostname => write!(f, "hostname"),
            Self::Url => write!(f, "url"),
            Self::Email => write!(f, "email"),
            Self::IpAddr => write!(f, "ip_addr"),
            Self::SocketAddr => write!(f, "socket_addr"),
            Self::AbsolutePath => write!(f, "absolute_path"),
        }
    }
}
