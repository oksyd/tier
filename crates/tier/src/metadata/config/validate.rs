mod check;
mod field;

use crate::ConfigError;

use self::check::validate_check;
use self::field::validate_field;
use super::super::ConfigMetadata;

impl ConfigMetadata {
    pub(crate) fn validate_paths(&self) -> Result<(), ConfigError> {
        let _ = self.env_overrides()?;

        for field in &self.fields {
            validate_field(self, field)?;
        }

        for check in &self.checks {
            validate_check(check)?;
        }

        Ok(())
    }
}
