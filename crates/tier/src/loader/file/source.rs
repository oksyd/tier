use std::path::PathBuf;

use super::FileFormat;

#[derive(Debug, Clone)]
/// File-backed configuration source definition.
///
/// `FileSource` is useful when you need more control than
/// [`ConfigLoader::file`](crate::ConfigLoader::file) or
/// [`ConfigLoader::optional_file`](crate::ConfigLoader::optional_file)
/// provide, such as candidate-path search or explicit format selection.
///
/// # Examples
///
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use tier::{ConfigLoader, FileFormat, FileSource};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct AppConfig {
///     port: u16,
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self { port: 3000 }
///     }
/// }
///
/// let loaded = ConfigLoader::new(AppConfig::default())
///     .with_file(
///         FileSource::search(["config/local", "config/default.toml"]).format(FileFormat::Toml),
///     )
///     .load()?;
///
/// assert!(loaded.port >= 1);
/// # Ok::<(), tier::ConfigError>(())
/// ```
pub struct FileSource {
    pub(super) candidates: Vec<PathBuf>,
    pub(super) required: bool,
    pub(super) format: Option<FileFormat>,
}

impl FileSource {
    /// Creates a required file source for a single path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            candidates: vec![path.into()],
            required: true,
            format: None,
        }
    }

    /// Creates an optional file source for a single path.
    #[must_use]
    pub fn optional(path: impl Into<PathBuf>) -> Self {
        Self {
            candidates: vec![path.into()],
            required: false,
            format: None,
        }
    }

    /// Creates a required file source that searches candidate paths in order.
    #[must_use]
    pub fn search<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        Self {
            candidates: paths.into_iter().map(Into::into).collect(),
            required: true,
            format: None,
        }
    }

    /// Creates an optional file source that searches candidate paths in order.
    #[must_use]
    pub fn optional_search<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        Self {
            candidates: paths.into_iter().map(Into::into).collect(),
            required: false,
            format: None,
        }
    }

    /// Returns configured candidate paths in priority order.
    #[must_use]
    pub fn candidates(&self) -> &[PathBuf] {
        &self.candidates
    }

    /// Overrides format inference with an explicit file format.
    #[must_use]
    pub fn format(mut self, format: FileFormat) -> Self {
        self.format = Some(format);
        self
    }
}
