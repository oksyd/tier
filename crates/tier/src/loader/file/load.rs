use std::path::PathBuf;

use crate::ConfigError;

use super::parse::{infer_format, parse_file_value};
use super::profile::resolve_profile_path;
use super::source::FileSource;
use crate::loader::{Layer, SourceKind, SourceTrace};

pub(in crate::loader) fn load_file_layer(
    file: FileSource,
    profile: Option<&str>,
) -> Result<Option<Layer>, ConfigError> {
    let resolved_paths = resolve_candidate_paths(&file, profile)?;
    let Some(path) = first_existing_path(&resolved_paths) else {
        return missing_file_result(file.required, resolved_paths);
    };

    let content = std::fs::read_to_string(&path).map_err(|source| ConfigError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let format = file.format.map_or_else(|| infer_format(&path), Ok)?;
    let value = parse_file_value(&path, &content, format)?;

    let layer = Layer::from_value(
        SourceTrace::new(SourceKind::File, path.display().to_string()),
        value,
    )?;

    Ok(Some(layer))
}

fn resolve_candidate_paths(
    file: &FileSource,
    profile: Option<&str>,
) -> Result<Vec<PathBuf>, ConfigError> {
    file.candidates
        .iter()
        .map(|path| resolve_profile_path(path, profile))
        .collect()
}

fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| path.exists()).cloned()
}

fn missing_file_result(
    required: bool,
    resolved_paths: Vec<PathBuf>,
) -> Result<Option<Layer>, ConfigError> {
    if !required {
        return Ok(None);
    }

    match resolved_paths.as_slice() {
        [] => Err(ConfigError::InvalidArg {
            arg: "file source".to_owned(),
            message: "at least one candidate path must be provided".to_owned(),
        }),
        [single] => Err(ConfigError::MissingFile {
            path: single.clone(),
        }),
        _ => Err(ConfigError::MissingFiles {
            paths: resolved_paths,
        }),
    }
}
