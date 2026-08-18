use quote::{format_ident, quote};
use syn::{DataEnum, Fields, LitStr};

use crate::attr::parse_patch_attrs;
use crate::field::{named_field_ident, single_unnamed_field};
use crate::serde_attrs::{
    SerdeContainerAttrs, SerdeFieldContext, has_field_naming_attrs, parse_serde_variant_attrs,
};

use super::field::expand_patch_bound_field;
use super::tokens::generate_nested_patch_tokens;

pub(super) fn expand_patch_enum(
    data_enum: DataEnum,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut variant_arms = Vec::new();
    let context = SerdeFieldContext::for_enum_variant_fields(container_attrs);

    for variant in data_enum.variants {
        let variant_ident = variant.ident.clone();
        let serde_attrs =
            parse_serde_variant_attrs(&variant.attrs, &variant_ident, container_attrs)?;
        let attrs = parse_patch_attrs(&variant.attrs)?;

        if serde_attrs.skip_metadata {
            if attrs.has_non_skip() {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    "skipped variants cannot use tier patch attributes",
                ));
            }
            variant_arms.push(expand_noop_patch_variant_arm(
                &variant_ident,
                &variant.fields,
            ));
            continue;
        }

        if attrs.skip {
            if attrs.has_non_skip() {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    "skipped patch variants cannot use other tier patch attributes",
                ));
            }
            variant_arms.push(expand_noop_patch_variant_arm(
                &variant_ident,
                &variant.fields,
            ));
            continue;
        }

        if attrs.path.is_some() && attrs.path_expr.is_some() {
            return Err(syn::Error::new_spanned(
                variant_ident,
                "patch variants must use either tier(path = ...) or tier(path_expr = ...), not both",
            ));
        }

        let variant_prefix = if let Some(path_expr) = attrs.path_expr {
            quote! { ::tier::patch::join_patch_prefix(__tier_prefix, #path_expr) }
        } else if let Some(path) = attrs.path {
            let path_lit = LitStr::new(&path, variant_ident.span());
            quote! { ::tier::patch::join_patch_prefix(__tier_prefix, #path_lit) }
        } else {
            quote! { ::std::string::String::from(__tier_prefix) }
        };

        match variant.fields {
            Fields::Named(fields) => {
                let mut bindings = Vec::new();
                let mut body_tokens = Vec::new();
                for field in fields.named {
                    let binding_ident = named_field_ident(&field)?;
                    body_tokens.push(expand_patch_bound_field(
                        field,
                        context,
                        quote! { #binding_ident },
                    )?);
                    bindings.push(quote! { #binding_ident });
                }

                variant_arms.push(quote! {
                    Self::#variant_ident { #(#bindings),* } => {
                        let __tier_prefix = #variant_prefix;
                        #(#body_tokens)*
                    }
                });
            }
            Fields::Unnamed(fields) => {
                let field = single_unnamed_field(
                    fields,
                    "TierPatch only supports tuple variants with exactly one field",
                )?;
                if parse_patch_attrs(&field.attrs)?.has_any()
                    || has_field_naming_attrs(&field.attrs)?
                {
                    return Err(syn::Error::new_spanned(
                        field,
                        "tuple patch variants cannot use field-level tier or serde naming attributes",
                    ));
                }
                let binding_ident = format_ident!("__tier_variant_value");
                let body_token = generate_nested_patch_tokens(
                    &field.ty,
                    quote! { #binding_ident },
                    quote! { __tier_prefix.clone() },
                );

                variant_arms.push(quote! {
                    Self::#variant_ident(#binding_ident) => {
                        let __tier_prefix = #variant_prefix;
                        #body_token
                    }
                });
            }
            Fields::Unit => {
                variant_arms.push(quote! {
                    Self::#variant_ident => {}
                });
            }
        }
    }

    Ok(quote! {
        match self {
            #(#variant_arms),*
        }
    })
}

fn expand_noop_patch_variant_arm(
    variant_ident: &syn::Ident,
    fields: &Fields,
) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(_) => quote! { Self::#variant_ident { .. } => {} },
        Fields::Unnamed(_) => quote! { Self::#variant_ident(..) => {} },
        Fields::Unit => quote! { Self::#variant_ident => {} },
    }
}
