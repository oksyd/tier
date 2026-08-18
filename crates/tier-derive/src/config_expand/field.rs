use quote::quote;
use syn::{FieldsNamed, LitStr};

use crate::attr::parse_tier_attrs;
use crate::field::named_field_ident;
use crate::field_codegen::direct_field_metadata_tokens;
use crate::serde_attrs::{NonExternalFieldConflicts, SerdeFieldContext, parse_serde_field_attrs};
use crate::ty::{is_secret_type, metadata_target_type};

use super::docs::doc_comment;
use super::validate::{validate_merge_strategy, validate_validation_attrs};

pub(super) fn expand_named_fields_metadata(
    fields: FieldsNamed,
    context: SerdeFieldContext,
    accumulator: &proc_macro2::Ident,
    conflicts: Option<&NonExternalFieldConflicts>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut field_tokens = Vec::new();

    for field in fields.named {
        field_tokens.extend(expand_named_field_metadata(
            field,
            context,
            accumulator,
            conflicts,
        )?);
    }

    Ok(field_tokens)
}

fn expand_named_field_metadata(
    field: syn::Field,
    context: SerdeFieldContext,
    accumulator: &proc_macro2::Ident,
    conflicts: Option<&NonExternalFieldConflicts>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let field_ident = named_field_ident(&field)?;
    let mut serde_attrs = parse_serde_field_attrs(&field.attrs, &field_ident, context)?;
    let mut attrs = parse_tier_attrs(&field.attrs)?;
    if attrs.doc.is_none() {
        attrs.doc = doc_comment(&field.attrs);
    }

    if serde_attrs.skip_metadata {
        if attrs.has_any() {
            return Err(syn::Error::new_spanned(
                field_ident,
                "skipped fields cannot use tier metadata attributes",
            ));
        }
        return Ok(Vec::new());
    }

    if serde_attrs.flatten && attrs.has_any() {
        return Err(syn::Error::new_spanned(
            field_ident,
            "flattened fields cannot use tier metadata attributes",
        ));
    }

    if let Some(conflicts) = conflicts {
        if conflicts
            .skipped_fields
            .contains(&serde_attrs.canonical_name)
        {
            return Ok(Vec::new());
        }
        serde_attrs
            .aliases
            .retain(|alias| !conflicts.skipped_aliases.contains(alias));
        if attrs
            .env
            .as_ref()
            .is_some_and(|env| conflicts.skipped_envs.contains(env))
        {
            attrs.env = None;
        }
    }

    validate_merge_strategy(&attrs, &field.ty)?;
    validate_validation_attrs(&attrs, &field_ident)?;

    let field_type = field.ty;
    let metadata_ty = metadata_target_type(&field_type);
    let canonical_name_lit = LitStr::new(&serde_attrs.canonical_name, field_ident.span());
    let alias_lits = serde_attrs
        .aliases
        .iter()
        .map(|alias| LitStr::new(alias, field_ident.span()))
        .collect::<Vec<_>>();

    if serde_attrs.flatten {
        return Ok(vec![quote! {
            #accumulator.extend(<#metadata_ty as ::tier::TierMetadata>::metadata());
        }]);
    }

    let nested_metadata = if attrs.leaf {
        quote! { ::tier::ConfigMetadata::new() }
    } else {
        quote! { <#metadata_ty as ::tier::TierMetadata>::metadata() }
    };

    Ok(vec![
        quote! {
            #accumulator.extend(::tier::metadata::prefixed_metadata(
                #canonical_name_lit,
                ::std::vec![#(::std::string::String::from(#alias_lits)),*],
                #nested_metadata,
            ));
        },
        direct_field_metadata_tokens(
            accumulator,
            &canonical_name_lit,
            &alias_lits,
            &serde_attrs,
            &attrs,
            is_secret_type(metadata_ty),
        )?,
    ])
}
