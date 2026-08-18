use quote::quote;
use syn::{Field, FieldsNamed, LitStr};

use crate::attr::parse_patch_attrs;
use crate::field::named_field_ident;
use crate::serde_attrs::{SerdeFieldContext, parse_serde_field_attrs};

use super::tokens::{generate_leaf_patch_tokens, generate_nested_patch_tokens};

pub(super) fn expand_patch_fields_metadata(
    fields: FieldsNamed,
    context: SerdeFieldContext,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut field_tokens = Vec::new();

    for field in fields.named {
        field_tokens.push(expand_patch_field_metadata(field, context)?);
    }

    Ok(field_tokens)
}

fn expand_patch_field_metadata(
    field: Field,
    context: SerdeFieldContext,
) -> syn::Result<proc_macro2::TokenStream> {
    let field_ident = named_field_ident(&field)?;
    let field_access = quote! { &self.#field_ident };
    expand_patch_bound_field(field, context, field_access)
}

pub(super) fn expand_patch_bound_field(
    field: Field,
    context: SerdeFieldContext,
    field_access: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let field_ident = named_field_ident(&field)?;
    let serde_attrs = parse_serde_field_attrs(&field.attrs, &field_ident, context)?;
    let attrs = parse_patch_attrs(&field.attrs)?;

    if serde_attrs.skip_metadata {
        if attrs.has_non_skip() {
            return Err(syn::Error::new_spanned(
                field_ident,
                "skipped fields cannot use tier patch attributes",
            ));
        }
        return Ok(quote! {});
    }

    if attrs.skip {
        if attrs.has_non_skip() {
            return Err(syn::Error::new_spanned(
                field_ident,
                "skipped patch fields cannot use other tier patch attributes",
            ));
        }
        return Ok(quote! {});
    }

    if attrs.path.is_some() && attrs.path_expr.is_some() {
        return Err(syn::Error::new_spanned(
            field_ident,
            "patch fields must use either tier(path = ...) or tier(path_expr = ...), not both",
        ));
    }

    if serde_attrs.flatten && (attrs.path.is_some() || attrs.path_expr.is_some()) {
        return Err(syn::Error::new_spanned(
            field_ident,
            "flattened patch fields cannot override their tier path",
        ));
    }

    let path_expr = if serde_attrs.flatten {
        quote! { ::std::string::String::from(__tier_prefix) }
    } else if let Some(path_expr) = attrs.path_expr {
        quote! { ::tier::patch::join_patch_prefix(&__tier_prefix, #path_expr) }
    } else {
        let default_path = attrs
            .path
            .clone()
            .unwrap_or_else(|| serde_attrs.canonical_name.clone());
        let path_lit = LitStr::new(&default_path, field_ident.span());
        quote! { ::tier::patch::join_patch_prefix(&__tier_prefix, #path_lit) }
    };

    if serde_attrs.flatten || attrs.nested {
        return Ok(generate_nested_patch_tokens(
            &field.ty,
            field_access,
            path_expr,
        ));
    }

    Ok(generate_leaf_patch_tokens(
        &field.ty,
        field_access,
        path_expr,
    ))
}
