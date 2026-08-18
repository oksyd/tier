use quote::quote;
use syn::Type;

use crate::ty::{option_inner_type, patch_inner_type};

pub(super) fn generate_nested_patch_tokens(
    field_ty: &Type,
    field_access: proc_macro2::TokenStream,
    path_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if option_inner_type(field_ty).is_some() {
        quote! {
            if let ::std::option::Option::Some(value) = #field_access {
                let __tier_path = #path_expr;
                ::tier::TierPatch::write_layer(value, __tier_builder, &__tier_path)?;
            }
        }
    } else if patch_inner_type(field_ty).is_some() {
        quote! {
            if let ::std::option::Option::Some(value) = #field_access.as_ref() {
                let __tier_path = #path_expr;
                ::tier::TierPatch::write_layer(value, __tier_builder, &__tier_path)?;
            }
        }
    } else {
        quote! {
            {
                let __tier_path = #path_expr;
                ::tier::TierPatch::write_layer(#field_access, __tier_builder, &__tier_path)?;
            }
        }
    }
}

pub(super) fn generate_leaf_patch_tokens(
    field_ty: &Type,
    field_access: proc_macro2::TokenStream,
    path_expr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if option_inner_type(field_ty).is_some() {
        quote! {
            if let ::std::option::Option::Some(value) = #field_access {
                let __tier_path = #path_expr;
                __tier_builder.insert_serialized(&__tier_path, value)?;
            }
        }
    } else if patch_inner_type(field_ty).is_some() {
        quote! {
            if let ::std::option::Option::Some(value) = #field_access.as_ref() {
                let __tier_path = #path_expr;
                __tier_builder.insert_serialized(&__tier_path, value)?;
            }
        }
    } else {
        quote! {
            {
                let __tier_path = #path_expr;
                __tier_builder.insert_serialized(&__tier_path, #field_access)?;
            }
        }
    }
}
