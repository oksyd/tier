use crate::serde_attrs::model::{EnumRepresentation, SerdeContainerAttrs};

pub(crate) fn ensure_struct_container_attrs(
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<()> {
    if container_attrs.rename_all_fields_serialize.is_some()
        || container_attrs.rename_all_fields_deserialize.is_some()
    {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde(rename_all_fields = ...) is only supported on enums",
        ));
    }
    if container_attrs.tag.is_some()
        || container_attrs.content.is_some()
        || container_attrs.untagged
    {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde enum tagging attributes are not supported on structs",
        ));
    }
    Ok(())
}

pub(crate) fn enum_representation(
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<EnumRepresentation> {
    if container_attrs.untagged && container_attrs.tag.is_some() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde(untagged) cannot be combined with serde(tag = ...)",
        ));
    }
    if container_attrs.untagged && container_attrs.content.is_some() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde(untagged) cannot be combined with serde(content = ...)",
        ));
    }
    if container_attrs.content.is_some() && container_attrs.tag.is_none() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde(content = ...) requires serde(tag = ...)",
        ));
    }

    if container_attrs.untagged {
        return Ok(EnumRepresentation::Untagged);
    }

    match (&container_attrs.tag, &container_attrs.content) {
        (Some(tag), Some(content)) => Ok(EnumRepresentation::Adjacent {
            tag: tag.clone(),
            content: content.clone(),
        }),
        (Some(tag), None) => Ok(EnumRepresentation::Internal { tag: tag.clone() }),
        (None, None) => Ok(EnumRepresentation::External),
        (None, Some(_)) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "serde(content = ...) requires serde(tag = ...)",
        )),
    }
}
