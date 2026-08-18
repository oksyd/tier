use crate::serde_attrs::SerdeContainerAttrs;

pub(super) fn ensure_struct_patch_container_attrs(
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<()> {
    if container_attrs.rename_all_fields_serialize.is_some()
        || container_attrs.rename_all_fields_deserialize.is_some()
        || container_attrs.tag.is_some()
        || container_attrs.content.is_some()
        || container_attrs.untagged
    {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "TierPatch only supports struct-style serde container attributes",
        ));
    }

    Ok(())
}
