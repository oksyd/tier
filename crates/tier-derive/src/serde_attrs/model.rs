use std::collections::HashSet;

use super::rename::RenameRule;

#[derive(Debug, Default)]
pub(crate) struct SerdeContainerAttrs {
    pub(crate) rename_all_serialize: Option<RenameRule>,
    pub(crate) rename_all_deserialize: Option<RenameRule>,
    pub(crate) rename_all_fields_serialize: Option<RenameRule>,
    pub(crate) rename_all_fields_deserialize: Option<RenameRule>,
    pub(crate) default_fields: bool,
    pub(crate) tag: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) untagged: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SerdeFieldContext {
    pub(super) rename_serialize: Option<RenameRule>,
    pub(super) rename_deserialize: Option<RenameRule>,
    pub(super) default_fields: bool,
}

impl SerdeFieldContext {
    pub(crate) fn for_struct(container_attrs: &SerdeContainerAttrs) -> Self {
        Self {
            rename_serialize: container_attrs.rename_all_serialize,
            rename_deserialize: container_attrs.rename_all_deserialize,
            default_fields: container_attrs.default_fields,
        }
    }

    pub(crate) fn for_enum_variant_fields(container_attrs: &SerdeContainerAttrs) -> Self {
        Self {
            rename_serialize: container_attrs.rename_all_fields_serialize,
            rename_deserialize: container_attrs.rename_all_fields_deserialize,
            default_fields: false,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct SerdeFieldAttrs {
    pub(crate) canonical_name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) flatten: bool,
    pub(crate) skip_metadata: bool,
    pub(crate) has_default: bool,
}

#[derive(Debug, Default)]
pub(crate) struct SerdeVariantAttrs {
    pub(crate) canonical_name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) skip_metadata: bool,
}

#[derive(Debug, Default)]
pub(crate) struct NonExternalFieldConflicts {
    pub(crate) skipped_fields: HashSet<String>,
    pub(crate) skipped_aliases: HashSet<String>,
    pub(crate) skipped_envs: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum EnumRepresentation {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

impl EnumRepresentation {
    pub(crate) fn tag_field(&self) -> Option<&str> {
        match self {
            Self::Internal { tag } => Some(tag.as_str()),
            Self::Adjacent { tag, .. } => Some(tag.as_str()),
            Self::External | Self::Untagged => None,
        }
    }
}
