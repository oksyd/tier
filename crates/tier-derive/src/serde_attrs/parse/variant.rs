use syn::Attribute;

use crate::serde_attrs::model::{SerdeContainerAttrs, SerdeVariantAttrs};
use crate::syntax::{
    consume_unused_meta, parse_flag, parse_string_value, reject_duplicate_flag, unraw,
};

use super::rename_meta::parse_rename_meta;

pub(crate) fn parse_serde_variant_attrs(
    attributes: &[Attribute],
    variant_ident: &syn::Ident,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<SerdeVariantAttrs> {
    let base_name = unraw(variant_ident);
    let mut rename_serialize = None;
    let mut rename_deserialize = None;
    let mut aliases = Vec::new();
    let mut skip_metadata = false;

    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                parse_rename_meta(meta, &mut rename_serialize, &mut rename_deserialize)?;
                return Ok(());
            }
            if meta.path.is_ident("alias") {
                aliases.push(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("skip")
                || meta.path.is_ident("skip_deserializing")
                || meta.path.is_ident("other")
            {
                reject_duplicate_flag(
                    skip_metadata,
                    &meta,
                    "serde attribute `skip`, `skip_deserializing`, or `other`",
                )?;
                parse_flag(meta)?;
                skip_metadata = true;
                return Ok(());
            }
            consume_unused_meta(meta)?;
            Ok(())
        })?;
    }

    let canonical_name = rename_serialize
        .or_else(|| {
            container_attrs
                .rename_all_serialize
                .map(|rule| rule.apply_to_variant(&base_name))
        })
        .unwrap_or_else(|| base_name.clone());
    let deserialize_name = rename_deserialize
        .or_else(|| {
            container_attrs
                .rename_all_deserialize
                .map(|rule| rule.apply_to_variant(&base_name))
        })
        .unwrap_or_else(|| base_name.clone());

    if deserialize_name != canonical_name {
        aliases.push(deserialize_name);
    }

    aliases.retain(|alias| alias != &canonical_name);
    aliases.sort();
    aliases.dedup();

    Ok(SerdeVariantAttrs {
        canonical_name,
        aliases,
        skip_metadata,
    })
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, Data, DeriveInput};

    use super::parse_serde_variant_attrs;
    use crate::serde_attrs::SerdeContainerAttrs;

    fn variant_attrs(attribute: &str) -> (syn::Ident, Vec<Attribute>) {
        let input =
            syn::parse_str::<DeriveInput>(&format!("enum Config {{ {attribute} Variant }}"))
                .expect("test enum parses");
        let Data::Enum(data) = input.data else {
            unreachable!("test input is an enum");
        };
        let variant = data
            .variants
            .into_iter()
            .next()
            .expect("test input has a variant");
        (variant.ident, variant.attrs)
    }

    #[test]
    fn serde_variant_flags_reject_values_when_they_change_tier_metadata() {
        let (ident, attrs) = variant_attrs("#[serde(other = false)]");
        let error = parse_serde_variant_attrs(&attrs, &ident, &SerdeContainerAttrs::default())
            .expect_err("serde other values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn serde_variant_rejects_duplicate_metadata_flags() {
        let (ident, attrs) = variant_attrs("#[serde(skip, other)]");
        let error = parse_serde_variant_attrs(&attrs, &ident, &SerdeContainerAttrs::default())
            .expect_err("duplicate serde variant metadata flags should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `skip`, `skip_deserializing`, or `other`")
        );
    }
}
