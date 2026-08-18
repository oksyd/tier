use crate::ConfigError;
use crate::metadata::ValidationCheck;
use crate::metadata::paths::validate_check_path;

pub(super) fn validate_check(check: &ValidationCheck) -> Result<(), ConfigError> {
    match check {
        ValidationCheck::AtLeastOneOf { paths }
        | ValidationCheck::ExactlyOneOf { paths }
        | ValidationCheck::MutuallyExclusive { paths } => {
            for path in paths {
                validate_check_path(path)?;
            }
        }
        ValidationCheck::RequiredWith { path, requires }
        | ValidationCheck::RequiredIf { path, requires, .. } => {
            validate_check_path(path)?;
            for required in requires {
                validate_check_path(required)?;
            }
        }
    }

    Ok(())
}
