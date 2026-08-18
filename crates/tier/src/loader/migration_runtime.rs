use std::collections::BTreeSet;

use serde_json::Value;

use crate::error::ConfigError;
use crate::path::{get_value_at_path, parse_array_index_segment};
use crate::report::{AppliedMigration, ConfigReport};

use super::SourceKind;
use super::canonical::{
    canonicalize_runtime_path, try_canonicalize_runtime_path_with_explicit_arrays,
};
use super::de::insert_path_with_shape_and_explicit_arrays;
use super::migration::{ConfigMigration, ConfigMigrationKind, MigrationConflictPolicy};
use super::path::try_normalize_external_path_with_explicit_arrays;

#[derive(Debug, Clone)]
pub(super) struct MigrationPathSpec {
    path: String,
    explicit_array_segments: BTreeSet<usize>,
}

pub(super) fn normalize_version_registration_path(
    path: &str,
) -> Result<MigrationPathSpec, ConfigError> {
    let spec = normalize_registration_path(path, "configuration version path")?;
    if spec.path.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: "configuration version path cannot be empty".to_owned(),
        });
    }
    Ok(spec)
}

pub(super) fn validate_config_migrations(
    migrations: &[ConfigMigration],
) -> Result<(), ConfigError> {
    for migration in migrations {
        match &migration.kind {
            ConfigMigrationKind::Rename { from, to, .. } => {
                let _ = normalize_migration_registration_path(from)?;
                let _ = normalize_migration_registration_path(to)?;
            }
            ConfigMigrationKind::Remove { path } => {
                let _ = normalize_migration_registration_path(path)?;
            }
        }
    }

    Ok(())
}

pub(super) fn apply_config_migrations(
    merged: &mut Value,
    version_path: &MigrationPathSpec,
    current_version: u32,
    migrations: &[ConfigMigration],
    report: &mut ConfigReport,
) -> Result<(), ConfigError> {
    let version_path =
        canonicalize_migration_path(merged, version_path, "configuration version path")?;
    let mut working_version = read_config_version(merged, &version_path.path)?;
    if working_version > current_version {
        return Err(ConfigError::UnsupportedConfigVersion {
            path: version_path.path,
            found: working_version,
            supported: current_version,
        });
    }

    let mut sorted = migrations.to_vec();
    sorted.sort_by_key(|migration| migration.since_version);

    let mut migration_index = 0;
    while migration_index < sorted.len() {
        let since_version = sorted[migration_index].since_version;
        let group_start = migration_index;
        while migration_index < sorted.len()
            && sorted[migration_index].since_version == since_version
        {
            migration_index += 1;
        }

        if since_version <= working_version || since_version > current_version {
            continue;
        }

        let from_version = working_version;
        for migration in &sorted[group_start..migration_index] {
            apply_config_migration(merged, migration, from_version, report)?;
        }

        working_version = since_version;
    }

    insert_normalized_path(
        merged,
        &version_path.path,
        &version_path.explicit_array_segments,
        Value::Number(serde_json::Number::from(current_version)),
    )
    .map_err(|message| ConfigError::InvalidConfigVersion {
        path: version_path.path,
        message,
    })?;

    Ok(())
}

fn apply_config_migration(
    merged: &mut Value,
    migration: &ConfigMigration,
    from_version: u32,
    report: &mut ConfigReport,
) -> Result<(), ConfigError> {
    match &migration.kind {
        ConfigMigrationKind::Rename {
            from,
            to,
            conflict_policy,
        } => {
            let from = normalize_migration_registration_path(from)?;
            let to = normalize_migration_registration_path(to)?;
            let from = canonicalize_migration_path(merged, &from, "migration path")?;
            let to = canonicalize_migration_path(merged, &to, "migration path")?;
            if get_value_at_path(merged, &from.path).is_none() {
                return Ok(());
            }
            reject_array_element_rename(merged, &from.path)?;
            reject_array_element_rename(merged, &to.path)?;

            let target_provenance = get_value_at_path(merged, &to.path)
                .and_then(|_| report.latest_source_for(&to.path))
                .filter(|source| source.kind != SourceKind::Default);
            let has_explicit_target = target_provenance.is_some();
            if has_explicit_target && matches!(conflict_policy, MigrationConflictPolicy::Error) {
                return Err(ConfigError::MigrationConflict {
                    from_path: from.path,
                    to_path: to.path,
                    provenance: target_provenance,
                });
            }

            let value = take_value_at_path(merged, &from.path);
            if !(has_explicit_target
                && matches!(conflict_policy, MigrationConflictPolicy::KeepTarget))
                && let Some(value) = value
            {
                insert_normalized_path(merged, &to.path, &to.explicit_array_segments, value)
                    .map_err(|message| ConfigError::MetadataInvalid {
                        path: to.path.clone(),
                        message: format!("failed to apply migration: {message}"),
                    })?;
            }
            report.record_migration(AppliedMigration {
                kind: "rename".to_owned(),
                from_version,
                to_version: migration.since_version,
                from_path: from.path,
                to_path: Some(to.path),
                note: migration.note.clone(),
            });
        }
        ConfigMigrationKind::Remove { path } => {
            let path = normalize_migration_registration_path(path)?;
            let path = canonicalize_migration_path(merged, &path, "migration path")?;
            if take_value_at_path(merged, &path.path).is_some() {
                report.record_migration(AppliedMigration {
                    kind: "remove".to_owned(),
                    from_version,
                    to_version: migration.since_version,
                    from_path: path.path,
                    to_path: None,
                    note: migration.note.clone(),
                });
            }
        }
    }
    Ok(())
}

fn reject_array_element_rename(value: &Value, path: &str) -> Result<(), ConfigError> {
    let mut segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some(last) = segments.pop() else {
        return Ok(());
    };
    let parent_path = segments.join(".");
    if matches!(
        get_value_at_path(value, &parent_path),
        Some(Value::Array(_))
    ) {
        return Err(ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: format!(
                "rename migrations cannot move entire array element `{last}`; migrate fields within the element or replace the array explicitly"
            ),
        });
    }
    Ok(())
}

fn normalize_migration_registration_path(path: &str) -> Result<MigrationPathSpec, ConfigError> {
    let spec = normalize_registration_path(path, "migration path")?;
    if spec.path.is_empty() {
        return Err(ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: "migration paths cannot target the configuration root".to_owned(),
        });
    }
    Ok(spec)
}

fn normalize_registration_path(path: &str, kind: &str) -> Result<MigrationPathSpec, ConfigError> {
    let (path, explicit_array_segments) = try_normalize_external_path_with_explicit_arrays(path)
        .map_err(|message| ConfigError::MetadataInvalid {
            path: path.to_owned(),
            message: format!("invalid {kind}: {message}"),
        })?;
    Ok(MigrationPathSpec {
        path,
        explicit_array_segments,
    })
}

fn canonicalize_migration_path(
    value: &Value,
    spec: &MigrationPathSpec,
    kind: &str,
) -> Result<MigrationPathSpec, ConfigError> {
    if spec.explicit_array_segments.is_empty() {
        return Ok(MigrationPathSpec {
            path: canonicalize_runtime_path(value, &spec.path),
            explicit_array_segments: BTreeSet::new(),
        });
    }

    let path = try_canonicalize_runtime_path_with_explicit_arrays(
        value,
        &spec.path,
        &spec.explicit_array_segments,
    )
    .map_err(|message| ConfigError::MetadataInvalid {
        path: spec.path.clone(),
        message: format!("invalid {kind}: {message}"),
    })?;
    Ok(MigrationPathSpec {
        path,
        explicit_array_segments: spec.explicit_array_segments.clone(),
    })
}

fn read_config_version(value: &Value, path: &str) -> Result<u32, ConfigError> {
    let Some(found) = get_value_at_path(value, path) else {
        return Ok(0);
    };

    let Some(version) = found.as_u64() else {
        return Err(ConfigError::InvalidConfigVersion {
            path: path.to_owned(),
            message: "expected an unsigned integer".to_owned(),
        });
    };

    u32::try_from(version).map_err(|_| ConfigError::InvalidConfigVersion {
        path: path.to_owned(),
        message: "version must fit in u32".to_owned(),
    })
}

fn insert_normalized_path(
    root: &mut Value,
    path: &str,
    explicit_array_segments: &BTreeSet<usize>,
    value: Value,
) -> Result<(), String> {
    let segments = path.split('.').collect::<Vec<_>>();
    insert_path_with_shape_and_explicit_arrays(
        root,
        None,
        &segments,
        explicit_array_segments,
        value,
    )
}

fn take_value_at_path(root: &mut Value, path: &str) -> Option<Value> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    take_value_at_segments(root, &segments)
}

fn take_value_at_segments(current: &mut Value, segments: &[&str]) -> Option<Value> {
    let segment = segments.first()?;
    if segments.len() == 1 {
        return match current {
            Value::Object(map) => map.remove(*segment),
            Value::Array(values) => {
                let index = parse_array_index_segment(segment).ok()?;
                (index < values.len()).then(|| values.remove(index))
            }
            _ => None,
        };
    }

    match current {
        Value::Object(map) => {
            let child = map.get_mut(*segment)?;
            take_value_at_segments(child, &segments[1..])
        }
        Value::Array(values) => {
            let index = parse_array_index_segment(segment).ok()?;
            let child = values.get_mut(index)?;
            take_value_at_segments(child, &segments[1..])
        }
        _ => None,
    }
}
