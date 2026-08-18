use std::fmt::{self, Display, Formatter};

use super::super::paths::{
    join_explicit_array_segments, normalize_check_path, normalize_check_path_group_specs,
    path_spec_to_public_path,
};
use super::super::{MetadataPathSpec, ValidationCheck, ValidationCheckSpec};

impl ValidationCheck {
    /// Returns a stable machine-readable rule identifier.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::AtLeastOneOf { .. } => "at_least_one_of",
            Self::ExactlyOneOf { .. } => "exactly_one_of",
            Self::MutuallyExclusive { .. } => "mutually_exclusive",
            Self::RequiredWith { .. } => "required_with",
            Self::RequiredIf { .. } => "required_if",
        }
    }
}

impl ValidationCheckSpec {
    pub(in crate::metadata) fn from_public(check: ValidationCheck) -> Option<Self> {
        match check {
            ValidationCheck::AtLeastOneOf { paths } => {
                normalize_check_path_group_specs(paths).map(|paths| Self::AtLeastOneOf { paths })
            }
            ValidationCheck::ExactlyOneOf { paths } => {
                normalize_check_path_group_specs(paths).map(|paths| Self::ExactlyOneOf { paths })
            }
            ValidationCheck::MutuallyExclusive { paths } => normalize_check_path_group_specs(paths)
                .map(|paths| Self::MutuallyExclusive { paths }),
            ValidationCheck::RequiredWith { path, requires } => {
                let path = normalize_check_path(&path);
                let requires = normalize_check_path_group_specs(requires)?;
                Some(Self::RequiredWith { path, requires })
            }
            ValidationCheck::RequiredIf {
                path,
                equals,
                requires,
            } => {
                let path = normalize_check_path(&path);
                let requires = normalize_check_path_group_specs(requires)?;
                Some(Self::RequiredIf {
                    path,
                    equals,
                    requires,
                })
            }
        }
    }

    pub(crate) fn to_public(&self) -> ValidationCheck {
        match self {
            Self::AtLeastOneOf { paths } => ValidationCheck::AtLeastOneOf {
                paths: path_specs_to_public(paths),
            },
            Self::ExactlyOneOf { paths } => ValidationCheck::ExactlyOneOf {
                paths: path_specs_to_public(paths),
            },
            Self::MutuallyExclusive { paths } => ValidationCheck::MutuallyExclusive {
                paths: path_specs_to_public(paths),
            },
            Self::RequiredWith { path, requires } => ValidationCheck::RequiredWith {
                path: path_spec_to_public_path(path),
                requires: path_specs_to_public(requires),
            },
            Self::RequiredIf {
                path,
                equals,
                requires,
            } => ValidationCheck::RequiredIf {
                path: path_spec_to_public_path(path),
                equals: equals.clone(),
                requires: path_specs_to_public(requires),
            },
        }
    }

    pub(in crate::metadata) fn prefixed(self, prefix: &MetadataPathSpec) -> Option<Self> {
        if prefix.path.is_empty() {
            return Some(self);
        }

        match self {
            Self::AtLeastOneOf { paths } => Some(Self::AtLeastOneOf {
                paths: prefix_path_specs(paths, prefix),
            })
            .and_then(Self::normalize_specs),
            Self::ExactlyOneOf { paths } => Some(Self::ExactlyOneOf {
                paths: prefix_path_specs(paths, prefix),
            })
            .and_then(Self::normalize_specs),
            Self::MutuallyExclusive { paths } => Some(Self::MutuallyExclusive {
                paths: prefix_path_specs(paths, prefix),
            })
            .and_then(Self::normalize_specs),
            Self::RequiredWith { path, requires } => Some(Self::RequiredWith {
                path: prefix_path_spec(path, prefix),
                requires: prefix_path_specs(requires, prefix),
            })
            .and_then(Self::normalize_specs),
            Self::RequiredIf {
                path,
                equals,
                requires,
            } => Some(Self::RequiredIf {
                path: prefix_path_spec(path, prefix),
                equals,
                requires: prefix_path_specs(requires, prefix),
            })
            .and_then(Self::normalize_specs),
        }
    }

    pub(in crate::metadata) fn normalize_specs(self) -> Option<Self> {
        match self {
            Self::AtLeastOneOf { paths } if !paths.is_empty() => Some(Self::AtLeastOneOf { paths }),
            Self::ExactlyOneOf { paths } if !paths.is_empty() => Some(Self::ExactlyOneOf { paths }),
            Self::MutuallyExclusive { paths } if !paths.is_empty() => {
                Some(Self::MutuallyExclusive { paths })
            }
            Self::RequiredWith { path, requires } if !requires.is_empty() => {
                Some(Self::RequiredWith { path, requires })
            }
            Self::RequiredIf {
                path,
                equals,
                requires,
            } if !requires.is_empty() => Some(Self::RequiredIf {
                path,
                equals,
                requires,
            }),
            _ => None,
        }
    }
}

pub(crate) fn normalize_check_specs<I>(checks: I) -> Vec<ValidationCheckSpec>
where
    I: IntoIterator<Item = ValidationCheckSpec>,
{
    let mut normalized = Vec::new();
    for check in checks {
        let Some(check) = check.normalize_specs() else {
            continue;
        };
        if !normalized.contains(&check) {
            normalized.push(check);
        }
    }
    normalized
}

fn path_specs_to_public(paths: &[MetadataPathSpec]) -> Vec<String> {
    paths.iter().map(path_spec_to_public_path).collect()
}

fn prefix_path_specs(
    paths: Vec<MetadataPathSpec>,
    prefix: &MetadataPathSpec,
) -> Vec<MetadataPathSpec> {
    paths
        .into_iter()
        .map(|path| prefix_path_spec(path, prefix))
        .collect()
}

fn prefix_path_spec(path: MetadataPathSpec, prefix: &MetadataPathSpec) -> MetadataPathSpec {
    let prefix_segments = prefix
        .path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .count();
    MetadataPathSpec {
        path: if path.path.is_empty() {
            prefix.path.clone()
        } else {
            format!("{}.{}", prefix.path, path.path)
        },
        explicit_array_segments: join_explicit_array_segments(
            &prefix.explicit_array_segments,
            prefix_segments,
            &path.explicit_array_segments,
        ),
    }
}

impl Display for ValidationCheck {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtLeastOneOf { paths } => {
                write!(f, "at_least_one_of({})", paths.join(", "))
            }
            Self::ExactlyOneOf { paths } => {
                write!(f, "exactly_one_of({})", paths.join(", "))
            }
            Self::MutuallyExclusive { paths } => {
                write!(f, "mutually_exclusive({})", paths.join(", "))
            }
            Self::RequiredWith { path, requires } => {
                write!(f, "required_with({path} -> {})", requires.join(", "))
            }
            Self::RequiredIf {
                path,
                equals,
                requires,
            } => write!(
                f,
                "required_if({path} == {equals} -> {})",
                requires.join(", ")
            ),
        }
    }
}
