use quote::quote;
use syn::{DataStruct, Fields};

use crate::serde_attrs::{SerdeContainerAttrs, SerdeFieldContext};

use super::field::expand_patch_fields_metadata;
use super::validate::ensure_struct_patch_container_attrs;

pub(super) fn expand_patch_struct(
    data_struct: DataStruct,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<proc_macro2::TokenStream> {
    ensure_struct_patch_container_attrs(container_attrs)?;

    let field_tokens = match data_struct.fields {
        Fields::Named(fields) => {
            expand_patch_fields_metadata(fields, SerdeFieldContext::for_struct(container_attrs))?
        }
        Fields::Unnamed(fields) => {
            return Err(syn::Error::new_spanned(
                fields,
                "TierPatch only supports structs with named fields",
            ));
        }
        Fields::Unit => Vec::new(),
    };

    Ok(quote! {
        #(#field_tokens)*
    })
}
