#[cfg(feature = "schema")]
use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Strongly typed secret wrapper.
///
/// `Secret<T>` keeps config ergonomics while preventing accidental leaks in
/// `Debug`, `Display`, and serde serialization.
///
/// Secret values intentionally do not implement [`Serialize`]. Exposing a
/// secret to a serialized output must be explicit at the field declaration:
///
/// ```rust
/// use serde::Serialize;
/// use tier::Secret;
///
/// #[derive(Serialize)]
/// struct Credentials {
///     #[serde(serialize_with = "tier::secret::serialize_exposed")]
///     token: Secret<String>,
/// }
/// ```
///
/// Accidental serialization is rejected at compile time:
///
/// ```compile_fail
/// let secret = tier::Secret::new("do-not-export".to_owned());
/// let _ = serde_json::to_string(&secret);
/// ```
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps a sensitive value.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Returns a shared reference to the inner value.
    #[must_use]
    pub fn expose_ref(&self) -> &T {
        &self.0
    }

    /// Returns a mutable reference to the inner value.
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Unwraps the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Debug for Secret<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl<T> Display for Secret<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("***redacted***")
    }
}

/// Explicitly serializes the inner value of a [`Secret`].
///
/// Use this function only through serde's `serialize_with` field attribute so
/// every plaintext export remains visible and auditable at its call site.
pub fn serialize_exposed<T, S>(value: &Secret<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    value.expose_ref().serialize(serializer)
}

impl<'de, T> Deserialize<'de> for Secret<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self)
    }
}

#[cfg(feature = "schema")]
impl<T> schemars::JsonSchema for Secret<T>
where
    T: schemars::JsonSchema,
{
    fn inline_schema() -> bool {
        T::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        Cow::Owned(format!("Secret_{}", T::schema_name()))
    }

    fn schema_id() -> Cow<'static, str> {
        Cow::Owned(format!("tier::Secret<{}>", T::schema_id()))
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = T::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert("writeOnly".to_owned(), serde_json::Value::Bool(true));
            object.insert("x-tier-secret".to_owned(), serde_json::Value::Bool(true));
        }
        schema
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    struct NonDebugSecret;

    #[test]
    fn secret_debug_does_not_require_inner_debug() {
        let secret = Secret::new(NonDebugSecret);

        assert_eq!(format!("{secret:?}"), "Secret(***)");
    }
}
