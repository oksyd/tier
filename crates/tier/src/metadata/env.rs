#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
/// Built-in decoders for structured environment variable values.
///
/// These decoders are intended for operational formats that are common in
/// deployments but inconvenient to express as JSON.
///
/// # Examples
///
/// ```
/// use tier::{ConfigMetadata, EnvDecoder, FieldMetadata};
///
/// let mut metadata = ConfigMetadata::new();
/// metadata.push(FieldMetadata::new("no_proxy").env_decoder(EnvDecoder::Csv));
/// metadata.push(FieldMetadata::new("labels").env_decoder(EnvDecoder::KeyValueMap));
///
/// assert_eq!(metadata.fields().len(), 2);
/// ```
pub enum EnvDecoder {
    /// Comma-separated values such as `a,b,c`, with quoted CSV fields supported.
    Csv,
    /// Platform-native path list syntax such as `PATH`.
    PathList,
    /// Comma-separated `key=value` pairs such as `a=1,b=2`, with quoted entries
    /// and quoted values supported.
    KeyValueMap,
    /// Whitespace-separated values such as `a b c`.
    Whitespace,
}
