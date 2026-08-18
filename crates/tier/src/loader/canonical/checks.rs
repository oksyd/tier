use serde_json::Value;

use crate::ConfigError;
use crate::metadata::{AliasOverrideSpec, ConfigMetadata, MetadataPathSpec, ValidationCheckSpec};

use super::super::Layer;
use super::metadata::{
    canonicalize_metadata_path_against_layers_with_explicit_arrays,
    canonicalize_metadata_path_against_value_with_explicit_arrays,
};

type CheckResult = Result<ValidationCheckSpec, ConfigError>;

fn canonicalize_metadata_path_specs_against_layers<I>(
    paths: I,
    layers: &[Layer],
) -> Result<Vec<MetadataPathSpec>, ConfigError>
where
    I: IntoIterator<Item = MetadataPathSpec>,
{
    canonicalize_metadata_path_specs(paths, |path| {
        canonicalize_metadata_path_against_layers_with_explicit_arrays(
            &path.path,
            &path.explicit_array_segments,
            layers,
        )
        .map(|canonical| MetadataPathSpec {
            path: canonical,
            explicit_array_segments: path.explicit_array_segments,
        })
    })
}

fn canonicalize_metadata_path_specs_against_value<I>(
    paths: I,
    value: &Value,
) -> Result<Vec<MetadataPathSpec>, ConfigError>
where
    I: IntoIterator<Item = MetadataPathSpec>,
{
    canonicalize_metadata_path_specs(paths, |path| {
        canonicalize_metadata_path_against_value_with_explicit_arrays(
            &path.path,
            &path.explicit_array_segments,
            value,
        )
        .map(|canonical| MetadataPathSpec {
            path: canonical,
            explicit_array_segments: path.explicit_array_segments,
        })
    })
}

fn canonicalize_metadata_path_specs<I, F>(
    paths: I,
    mut canonicalize: F,
) -> Result<Vec<MetadataPathSpec>, ConfigError>
where
    I: IntoIterator<Item = MetadataPathSpec>,
    F: FnMut(MetadataPathSpec) -> Result<MetadataPathSpec, ConfigError>,
{
    let mut canonicalized = Vec::new();
    for path in paths {
        let canonical = canonicalize(path)?;
        if canonical.path.is_empty() || canonicalized.contains(&canonical) {
            continue;
        }
        canonicalized.push(canonical);
    }
    Ok(canonicalized)
}

pub(in crate::loader) fn canonicalize_check_with_alias_specs(
    check: ValidationCheckSpec,
    aliases: &[AliasOverrideSpec],
    shape: Option<&Value>,
) -> ValidationCheckSpec {
    match check {
        ValidationCheckSpec::AtLeastOneOf { paths } => ValidationCheckSpec::AtLeastOneOf {
            paths: canonicalize_path_specs_with_aliases(paths, aliases, shape),
        },
        ValidationCheckSpec::ExactlyOneOf { paths } => ValidationCheckSpec::ExactlyOneOf {
            paths: canonicalize_path_specs_with_aliases(paths, aliases, shape),
        },
        ValidationCheckSpec::MutuallyExclusive { paths } => {
            ValidationCheckSpec::MutuallyExclusive {
                paths: canonicalize_path_specs_with_aliases(paths, aliases, shape),
            }
        }
        ValidationCheckSpec::RequiredWith { path, requires } => ValidationCheckSpec::RequiredWith {
            path: canonicalize_path_spec_with_aliases(path, aliases, shape),
            requires: canonicalize_path_specs_with_aliases(requires, aliases, shape),
        },
        ValidationCheckSpec::RequiredIf {
            path,
            equals,
            requires,
        } => ValidationCheckSpec::RequiredIf {
            path: canonicalize_path_spec_with_aliases(path, aliases, shape),
            equals,
            requires: canonicalize_path_specs_with_aliases(requires, aliases, shape),
        },
    }
}

fn canonicalize_path_specs_with_aliases(
    paths: Vec<MetadataPathSpec>,
    aliases: &[AliasOverrideSpec],
    shape: Option<&Value>,
) -> Vec<MetadataPathSpec> {
    let mut canonicalized = Vec::new();
    for path in paths {
        let canonical = canonicalize_path_spec_with_aliases(path, aliases, shape);
        if canonical.path.is_empty() || canonicalized.contains(&canonical) {
            continue;
        }
        canonicalized.push(canonical);
    }
    canonicalized
}

fn canonicalize_path_spec_with_aliases(
    path: MetadataPathSpec,
    aliases: &[AliasOverrideSpec],
    shape: Option<&Value>,
) -> MetadataPathSpec {
    let (canonical, explicit_array_segments) =
        ConfigMetadata::canonicalize_path_with_alias_specs_and_array_segments(
            &path.path,
            &path.explicit_array_segments,
            aliases,
            shape,
        );
    MetadataPathSpec {
        path: canonical,
        explicit_array_segments,
    }
}

pub(in crate::loader) fn canonicalize_check_against_layers(
    check: ValidationCheckSpec,
    layers: &[Layer],
) -> CheckResult {
    match check {
        ValidationCheckSpec::AtLeastOneOf { paths } => Ok(ValidationCheckSpec::AtLeastOneOf {
            paths: canonicalize_metadata_path_specs_against_layers(paths, layers)?,
        }),
        ValidationCheckSpec::ExactlyOneOf { paths } => Ok(ValidationCheckSpec::ExactlyOneOf {
            paths: canonicalize_metadata_path_specs_against_layers(paths, layers)?,
        }),
        ValidationCheckSpec::MutuallyExclusive { paths } => {
            Ok(ValidationCheckSpec::MutuallyExclusive {
                paths: canonicalize_metadata_path_specs_against_layers(paths, layers)?,
            })
        }
        ValidationCheckSpec::RequiredWith { path, requires } => {
            Ok(ValidationCheckSpec::RequiredWith {
                path: canonicalize_metadata_path_specs_against_layers([path], layers)?
                    .into_iter()
                    .next()
                    .unwrap_or_else(empty_path_spec),
                requires: canonicalize_metadata_path_specs_against_layers(requires, layers)?,
            })
        }
        ValidationCheckSpec::RequiredIf {
            path,
            equals,
            requires,
        } => Ok(ValidationCheckSpec::RequiredIf {
            path: canonicalize_metadata_path_specs_against_layers([path], layers)?
                .into_iter()
                .next()
                .unwrap_or_else(empty_path_spec),
            equals,
            requires: canonicalize_metadata_path_specs_against_layers(requires, layers)?,
        }),
    }
}

pub(in crate::loader) fn canonicalize_check_against_value(
    check: ValidationCheckSpec,
    value: &Value,
) -> CheckResult {
    match check {
        ValidationCheckSpec::AtLeastOneOf { paths } => Ok(ValidationCheckSpec::AtLeastOneOf {
            paths: canonicalize_metadata_path_specs_against_value(paths, value)?,
        }),
        ValidationCheckSpec::ExactlyOneOf { paths } => Ok(ValidationCheckSpec::ExactlyOneOf {
            paths: canonicalize_metadata_path_specs_against_value(paths, value)?,
        }),
        ValidationCheckSpec::MutuallyExclusive { paths } => {
            Ok(ValidationCheckSpec::MutuallyExclusive {
                paths: canonicalize_metadata_path_specs_against_value(paths, value)?,
            })
        }
        ValidationCheckSpec::RequiredWith { path, requires } => {
            Ok(ValidationCheckSpec::RequiredWith {
                path: canonicalize_metadata_path_specs_against_value([path], value)?
                    .into_iter()
                    .next()
                    .unwrap_or_else(empty_path_spec),
                requires: canonicalize_metadata_path_specs_against_value(requires, value)?,
            })
        }
        ValidationCheckSpec::RequiredIf {
            path,
            equals,
            requires,
        } => Ok(ValidationCheckSpec::RequiredIf {
            path: canonicalize_metadata_path_specs_against_value([path], value)?
                .into_iter()
                .next()
                .unwrap_or_else(empty_path_spec),
            equals,
            requires: canonicalize_metadata_path_specs_against_value(requires, value)?,
        }),
    }
}

fn empty_path_spec() -> MetadataPathSpec {
    MetadataPathSpec {
        path: String::new(),
        explicit_array_segments: Default::default(),
    }
}
