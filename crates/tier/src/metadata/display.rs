use std::fmt::{self, Display, Formatter};

use super::{EnvDecoder, MergeStrategy, ValidationLevel};

impl Display for EnvDecoder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv => write!(f, "csv"),
            Self::PathList => write!(f, "path_list"),
            Self::KeyValueMap => write!(f, "key_value_map"),
            Self::Whitespace => write!(f, "whitespace"),
        }
    }
}

impl Display for MergeStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Merge => write!(f, "merge"),
            Self::Replace => write!(f, "replace"),
            Self::Append => write!(f, "append"),
        }
    }
}

impl Display for ValidationLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}
