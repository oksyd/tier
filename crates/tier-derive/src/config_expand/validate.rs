use std::cmp::Ordering;

use syn::Type;

use crate::attr::TierAttrs;
use crate::ty::supports_append_strategy;

pub(super) fn validate_merge_strategy(attrs: &TierAttrs, ty: &Type) -> syn::Result<()> {
    if attrs.merge.as_deref() == Some("append") && !supports_append_strategy(ty) {
        return Err(syn::Error::new_spanned(
            ty,
            "tier(merge = \"append\") requires a Vec<T> or array-like field",
        ));
    }
    Ok(())
}

pub(super) fn validate_validation_attrs(
    attrs: &TierAttrs,
    field_ident: &syn::Ident,
) -> syn::Result<()> {
    if let (Some(min), Some(max)) = (&attrs.min, &attrs.max) {
        match min.value.cmp_exact(&max.value) {
            Some(Ordering::Greater) => {
                return Err(syn::Error::new_spanned(
                    field_ident,
                    "tier(min = ...) cannot be greater than tier(max = ...)",
                ));
            }
            Some(_) => {}
            None => {
                return Err(syn::Error::new_spanned(
                    field_ident,
                    "tier(min = ...) and tier(max = ...) are too large to compare exactly",
                ));
            }
        }
    }

    if let (Some(min_length), Some(max_length)) = (attrs.min_length, attrs.max_length)
        && min_length > max_length
    {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(min_length = ...) cannot be greater than tier(max_length = ...)",
        ));
    }

    if let (Some(min_items), Some(max_items)) = (attrs.min_items, attrs.max_items)
        && min_items > max_items
    {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(min_items = ...) cannot be greater than tier(max_items = ...)",
        ));
    }

    if let (Some(min_properties), Some(max_properties)) =
        (attrs.min_properties, attrs.max_properties)
        && min_properties > max_properties
    {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(min_properties = ...) cannot be greater than tier(max_properties = ...)",
        ));
    }

    if let Some(multiple_of) = &attrs.multiple_of
        && !multiple_of.value.is_positive()
    {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(multiple_of = ...) must be greater than 0",
        ));
    }

    if attrs.pattern.as_deref() == Some("") {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(pattern = ...) cannot be empty",
        ));
    }

    if attrs.one_of.is_empty()
        && (attrs.hostname
            || attrs.url
            || attrs.email
            || attrs.ip_addr
            || attrs.socket_addr
            || attrs.absolute_path)
    {
        return Ok(());
    }

    if !attrs.one_of.is_empty() && (attrs.min.is_some() || attrs.max.is_some()) {
        return Err(syn::Error::new_spanned(
            field_ident,
            "tier(one_of(...)) cannot be combined with tier(min = ...) or tier(max = ...)",
        ));
    }

    Ok(())
}
