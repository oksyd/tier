use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::loader) struct SecretPathSpec {
    path: String,
    explicit_array_segments: BTreeSet<usize>,
}

impl SecretPathSpec {
    pub(in crate::loader) fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            explicit_array_segments: BTreeSet::new(),
        }
    }

    pub(in crate::loader) fn from_normalized(
        path: impl Into<String>,
        explicit_array_segments: BTreeSet<usize>,
    ) -> Self {
        Self {
            path: path.into(),
            explicit_array_segments,
        }
    }

    pub(in crate::loader) fn path(&self) -> &str {
        &self.path
    }

    pub(in crate::loader) fn explicit_array_segments(&self) -> &BTreeSet<usize> {
        &self.explicit_array_segments
    }

    pub(in crate::loader) fn into_path(self) -> String {
        self.path
    }
}
