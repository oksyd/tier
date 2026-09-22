use quote::{format_ident, quote};
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

    if !serde_attrs.flatten && attrs.doc.is_none() {
        attrs.doc = doc_comment(&field.attrs);
    }

    let mut security_tokens = Vec::new();
    if let Some(conflicts) = conflicts {
        if conflicts
            .skipped_fields
            .contains(&serde_attrs.canonical_name)
        {
            return expand_security_metadata(field, context, accumulator, &[]);
        }
        let discarded_aliases = serde_attrs
            .aliases
            .iter()
            .filter(|alias| conflicts.skipped_aliases.contains(*alias))
            .map(|alias| LitStr::new(alias, field_ident.span()))
            .collect::<Vec<_>>();
        if !discarded_aliases.is_empty() {
            security_tokens =
                expand_security_metadata(field.clone(), context, accumulator, &discarded_aliases)?;
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

    security_tokens.extend([
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
    ]);
    Ok(security_tokens)
}

fn expand_security_metadata(
    field: syn::Field,
    context: SerdeFieldContext,
    accumulator: &proc_macro2::Ident,
    only_aliases: &[LitStr],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let security_accumulator = format_ident!("__tier_security_metadata");
    let tokens = expand_named_field_metadata(field, context, &security_accumulator, None)?;
    let selected = if only_aliases.is_empty() {
        quote! { true }
    } else {
        quote! {
            [#(#only_aliases),*].iter().any(|alias| {
                __tier_path == *alias
                    || __tier_path.strip_prefix(alias).is_some_and(|suffix| suffix.starts_with('.'))
            })
        }
    };
    Ok(vec![quote! {
        {
            let mut #security_accumulator = ::tier::ConfigMetadata::new();
            #(#tokens)*
            for (__tier_path, __tier_field) in #security_accumulator.fields_by_path() {
                if __tier_field.is_secret()
                    || __tier_field.allowed_sources().is_some()
                    || __tier_field.denied_sources().is_some()
                {
                    // Protect accepted aliases without introducing ambiguous
                    // alias rewrites between enum variants.
                    for __tier_path in ::std::iter::once(__tier_path)
                        .chain(__tier_field.aliases().iter().cloned())
                    {
                        if #selected {
                            let mut __tier_security = ::tier::FieldMetadata::new(__tier_path);
                            if __tier_field.is_secret() {
                                __tier_security = __tier_security.secret();
                            }
                            if let Some(__tier_allowed) = __tier_field.allowed_sources() {
                                __tier_security = __tier_security.allow_sources(__tier_allowed.iter().copied());
                            }
                            if let Some(__tier_denied) = __tier_field.denied_sources() {
                                __tier_security = __tier_security.deny_sources(__tier_denied.iter().copied());
                            }
                            #accumulator.push(__tier_security);
                        }
                    }
                }
            }
        }
    }])
}
