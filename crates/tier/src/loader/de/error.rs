use serde::de::{Expected, Unexpected};
use std::fmt;

/// Retain the identity of borrowed strings across serde's flatten buffer, so a
/// retry can convert the rejected input without guessing other fields' types.
#[derive(Debug)]
pub(in crate::loader) struct ValueDeError {
    message: String,
    pub(in crate::loader) rejected_string: Option<usize>,
}
impl fmt::Display for ValueDeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ValueDeError {}
impl serde::de::Error for ValueDeError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self {
            message: message.to_string(),
            rejected_string: None,
        }
    }
    fn invalid_type(unexpected: Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self {
            message: format!("invalid type: {unexpected}, expected {expected}"),
            rejected_string: match unexpected {
                Unexpected::Str(raw) => Some(raw.as_ptr() as usize),
                _ => None,
            },
        }
    }
}
