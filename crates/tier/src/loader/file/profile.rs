use std::path::{Path, PathBuf};

use crate::ConfigError;

pub(super) fn resolve_profile_path(
    path: &Path,
    profile: Option<&str>,
) -> Result<PathBuf, ConfigError> {
    let raw = path.to_string_lossy();
    if raw.contains("{profile}") {
        let profile = profile.ok_or_else(|| ConfigError::MissingProfile {
            path: path.to_path_buf(),
        })?;
        Ok(PathBuf::from(raw.replace("{profile}", profile)))
    } else {
        Ok(path.to_path_buf())
    }
}
