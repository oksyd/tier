use quote::{format_ident, quote};
use syn::{DataEnum, Fields, FieldsUnnamed, LitStr};

use crate::attr::parse_tier_attrs;
use crate::field::single_unnamed_field;
use crate::serde_attrs::{
    EnumRepresentation, SerdeContainerAttrs, SerdeFieldContext, SerdeVariantAttrs,
    enum_representation, has_field_naming_attrs, non_external_variant_field_conflicts,
    parse_serde_variant_attrs,
};
use crate::ty::metadata_target_type;

use super::field::expand_named_fields_metadata;

pub(super) fn expand_enum_metadata(
    data_enum: DataEnum,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let representation = enum_representation(container_attrs)?;
    let conflicts = non_external_variant_field_conflicts(&data_enum, container_attrs)?;
    let mut tokens = vec![quote! {
        metadata.push(
            ::tier::FieldMetadata::new("").merge_strategy(::tier::MergeStrategy::Replace)
        );
    }];
    if let Some(tag) = representation.tag_field() {
        let tag_lit = LitStr::new(tag, proc_macro2::Span::call_site());
        tokens.push(quote! {
            metadata.push(::tier::FieldMetadata::new(#tag_lit));
        });
    }

    for variant in data_enum.variants {
        let variant_ident = variant.ident.clone();
        let variant_attrs =
            parse_serde_variant_attrs(&variant.attrs, &variant_ident, container_attrs)?;
        if variant_attrs.skip_metadata {
            continue;
        }

        match variant.fields {
            Fields::Named(fields) => {
                let field_tokens = expand_named_fields_metadata(
                    fields,
                    SerdeFieldContext::for_enum_variant_fields(container_attrs),
                    &format_ident!("variant_metadata"),
                    Some(&conflicts),
                )?;
                push_variant_tokens(
                    &mut tokens,
                    field_tokens,
                    &variant_attrs,
                    &representation,
                    variant_ident.span(),
                );
            }
            Fields::Unnamed(fields) => {
                let field_tokens = expand_newtype_variant_metadata(
                    fields,
                    &representation,
                    variant_ident.span(),
                    &format_ident!("variant_metadata"),
                )?;
                push_variant_tokens(
                    &mut tokens,
                    field_tokens,
                    &variant_attrs,
                    &representation,
                    variant_ident.span(),
                );
            }
            Fields::Unit => {}
        }
    }

    Ok(tokens)
}

fn push_variant_tokens(
    tokens: &mut Vec<proc_macro2::TokenStream>,
    variant_tokens: Vec<proc_macro2::TokenStream>,
    variant_attrs: &SerdeVariantAttrs,
    representation: &EnumRepresentation,
    span: proc_macro2::Span,
) {
    let variant_name_lit = LitStr::new(&variant_attrs.canonical_name, span);
    let variant_alias_lits = variant_attrs
        .aliases
        .iter()
        .map(|alias| LitStr::new(alias, span))
        .collect::<Vec<_>>();

    match representation {
        EnumRepresentation::External => {
            tokens.push(quote! {
                {
                    let mut variant_metadata = ::tier::ConfigMetadata::new();
                    #(#variant_tokens)*
                    metadata.extend(::tier::metadata::prefixed_metadata(
                        #variant_name_lit,
                        ::std::vec![#(::std::string::String::from(#variant_alias_lits)),*],
                        variant_metadata,
                    ));
                }
            });
        }
        EnumRepresentation::Adjacent { content, .. } => {
            let content_lit = LitStr::new(content, span);
            tokens.push(quote! {
                {
                    let mut variant_metadata = ::tier::ConfigMetadata::new();
                    #(#variant_tokens)*
                    metadata.extend(::tier::metadata::prefixed_metadata(
                        #content_lit,
                        ::std::vec![],
                        variant_metadata,
                    ));
                }
            });
        }
        EnumRepresentation::Internal { .. } | EnumRepresentation::Untagged => {
            tokens.push(quote! {
                {
                    let mut variant_metadata = ::tier::ConfigMetadata::new();
                    #(#variant_tokens)*
                    metadata.extend(variant_metadata);
                }
            });
        }
    }
}

fn expand_newtype_variant_metadata(
    fields: FieldsUnnamed,
    representation: &EnumRepresentation,
    span: proc_macro2::Span,
    accumulator: &proc_macro2::Ident,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let field = single_unnamed_field(
        fields,
        "TierConfig only supports enum tuple variants with exactly one field",
    )?;

    if matches!(representation, EnumRepresentation::Internal { .. }) {
        return Err(syn::Error::new(
            span,
            "internally tagged enums with tuple variants are not supported by TierConfig metadata",
        ));
    }

    if parse_tier_attrs(&field.attrs)?.has_any() || has_field_naming_attrs(&field.attrs)? {
        return Err(syn::Error::new_spanned(
            field,
            "tuple enum variants cannot use field-level tier or serde naming attributes",
        ));
    }

    let metadata_ty = metadata_target_type(&field.ty);
    Ok(vec![quote! {
        #accumulator.extend(<#metadata_ty as ::tier::TierMetadata>::metadata());
    }])
}
