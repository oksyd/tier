use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Supported on-disk configuration file formats.
pub enum FileFormat {
    /// JSON source file.
    Json,
    /// TOML source file.
    Toml,
    /// YAML source file.
    Yaml,
}

impl Display for FileFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Toml => write!(f, "toml"),
            Self::Yaml => write!(f, "yaml"),
        }
    }
}
