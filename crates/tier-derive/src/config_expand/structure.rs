use quote::format_ident;
use syn::{DataStruct, Fields, FieldsUnnamed};

use crate::attr::parse_tier_attrs;
use crate::field::single_unnamed_field;
use crate::serde_attrs::{
    SerdeContainerAttrs, SerdeFieldContext, ensure_struct_container_attrs, has_field_naming_attrs,
};
use crate::ty::metadata_target_type;

use super::field::expand_named_fields_metadata;

pub(super) fn expand_struct_metadata(
    data_struct: DataStruct,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    ensure_struct_container_attrs(container_attrs)?;

    match data_struct.fields {
        Fields::Named(fields) => expand_named_fields_metadata(
            fields,
            SerdeFieldContext::for_struct(container_attrs),
            &format_ident!("metadata"),
            None,
        ),
        Fields::Unnamed(fields) => {
            expand_newtype_struct_metadata(fields, &format_ident!("metadata"))
        }
        Fields::Unit => Ok(Vec::new()),
    }
}

fn expand_newtype_struct_metadata(
    fields: FieldsUnnamed,
    accumulator: &proc_macro2::Ident,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let field = single_unnamed_field(
        fields,
        "TierConfig only supports tuple structs with exactly one field",
    )?;
    if parse_tier_attrs(&field.attrs)?.has_any() || has_field_naming_attrs(&field.attrs)? {
        return Err(syn::Error::new_spanned(
            field,
            "tuple struct wrappers cannot use field-level tier or serde naming attributes",
        ));
    }

    let metadata_ty = metadata_target_type(&field.ty);
    Ok(vec![quote::quote! {
        #accumulator.extend(<#metadata_ty as ::tier::TierMetadata>::metadata());
    }])
}
