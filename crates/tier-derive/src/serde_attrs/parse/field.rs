use syn::Attribute;

use crate::serde_attrs::model::{SerdeFieldAttrs, SerdeFieldContext};
use crate::syntax::{
    consume_unused_meta, parse_bare_or_string_value, parse_flag, parse_string_value,
    reject_duplicate_flag, unraw,
};

use super::rename_meta::parse_rename_meta;

pub(crate) fn parse_serde_field_attrs(
    attributes: &[Attribute],
    field_ident: &syn::Ident,
    context: SerdeFieldContext,
) -> syn::Result<SerdeFieldAttrs> {
    let base_name = unraw(field_ident);
    let mut rename_serialize = None;
    let mut rename_deserialize = None;
    let mut aliases = Vec::new();
    let mut flatten = false;
    let mut skip_metadata = false;
    let mut explicit_default = false;
    let mut has_default = context.default_fields;

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
            if meta.path.is_ident("flatten") {
                reject_duplicate_flag(flatten, &meta, "serde attribute `flatten`")?;
                parse_flag(meta)?;
                flatten = true;
                return Ok(());
            }
            if meta.path.is_ident("default") {
                reject_duplicate_flag(explicit_default, &meta, "serde attribute `default`")?;
                explicit_default = true;
                has_default = true;
                parse_bare_or_string_value(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("skip") || meta.path.is_ident("skip_deserializing") {
                reject_duplicate_flag(
                    skip_metadata,
                    &meta,
                    "serde attribute `skip` or `skip_deserializing`",
                )?;
                parse_flag(meta)?;
                skip_metadata = true;
                return Ok(());
            }
            consume_unused_meta(meta)?;
            Ok(())
        })?;
    }

    let has_explicit_rename = rename_serialize.is_some() || rename_deserialize.is_some();

    let canonical_name = rename_serialize
        .or_else(|| {
            context
                .rename_serialize
                .map(|rule| rule.apply_to_field(&base_name))
        })
        .unwrap_or_else(|| base_name.clone());
    let deserialize_name = rename_deserialize
        .or_else(|| {
            context
                .rename_deserialize
                .map(|rule| rule.apply_to_field(&base_name))
        })
        .unwrap_or_else(|| base_name.clone());

    if deserialize_name != canonical_name {
        aliases.push(deserialize_name);
    }

    if flatten && (!aliases.is_empty() || has_explicit_rename) {
        return Err(syn::Error::new_spanned(
            field_ident,
            "flattened fields cannot use serde rename or alias attributes",
        ));
    }

    aliases.retain(|alias| alias != &canonical_name);
    aliases.sort();
    aliases.dedup();

    Ok(SerdeFieldAttrs {
        canonical_name,
        aliases,
        flatten,
        skip_metadata,
        has_default,
    })
}

pub(crate) fn has_field_naming_attrs(attributes: &[Attribute]) -> syn::Result<bool> {
    let mut has_naming = false;
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename")
                || meta.path.is_ident("alias")
                || meta.path.is_ident("flatten")
                || meta.path.is_ident("default")
            {
                has_naming = true;
            }
            if meta.path.is_ident("flatten") {
                parse_flag(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("default") {
                parse_bare_or_string_value(meta)?;
                return Ok(());
            }
            consume_unused_meta(meta)?;
            Ok(())
        })?;
    }

    Ok(has_naming)
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, Data, DeriveInput, Fields};

    use super::parse_serde_field_attrs;
    use crate::serde_attrs::{SerdeContainerAttrs, SerdeFieldContext};

    fn field_attrs(attribute: &str) -> (syn::Ident, Vec<Attribute>) {
        let input = syn::parse_str::<DeriveInput>(&format!(
            "struct Config {{ {attribute} field: String }}"
        ))
        .expect("test struct parses");
        let Data::Struct(data) = input.data else {
            unreachable!("test input is a struct");
        };
        let Fields::Named(fields) = data.fields else {
            unreachable!("test input has named fields");
        };
        let field = fields
            .named
            .into_iter()
            .next()
            .expect("test input has a field");
        (field.ident.expect("test input field is named"), field.attrs)
    }

    #[test]
    fn serde_flags_reject_values_when_they_change_tier_metadata() {
        let (ident, attrs) = field_attrs("#[serde(flatten = false)]");
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("serde flatten values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn serde_skip_rejects_values_when_it_changes_tier_metadata() {
        let (ident, attrs) = field_attrs("#[serde(skip_deserializing = false)]");
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("serde skip_deserializing values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn serde_rename_rejects_duplicate_single_value_forms() {
        let (ident, attrs) = field_attrs("#[serde(rename = \"a\", rename = \"b\")]");
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("duplicate serde rename should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `rename`")
        );
    }

    #[test]
    fn serde_rename_rejects_duplicate_directional_forms() {
        let (ident, attrs) = field_attrs(
            "#[serde(rename(serialize = \"a\", serialize = \"b\", deserialize = \"c\"))]",
        );
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("duplicate serde rename serialize should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `rename(serialize = ...)`")
        );
    }

    #[test]
    fn serde_field_rejects_duplicate_metadata_flags() {
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());

        let (ident, attrs) = field_attrs("#[serde(flatten, flatten)]");
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("duplicate serde flatten should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `flatten`")
        );

        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let (ident, attrs) = field_attrs("#[serde(skip, skip_deserializing)]");
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("duplicate serde skip metadata should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `skip` or `skip_deserializing`")
        );
    }

    #[test]
    fn serde_field_default_does_not_conflict_with_container_default() {
        let container_attrs = SerdeContainerAttrs {
            default_fields: true,
            ..SerdeContainerAttrs::default()
        };
        let context = SerdeFieldContext::for_struct(&container_attrs);
        let (ident, attrs) = field_attrs("#[serde(default)]");

        parse_serde_field_attrs(&attrs, &ident, context)
            .expect("field default can coexist with container default");
    }

    #[test]
    fn serde_field_default_accepts_only_bare_or_string_path_forms() {
        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let (ident, attrs) = field_attrs("#[serde(default = \"default_field\")]");
        parse_serde_field_attrs(&attrs, &ident, context)
            .expect("serde default string path should parse");

        let context = SerdeFieldContext::for_struct(&SerdeContainerAttrs::default());
        let (ident, attrs) = field_attrs("#[serde(default = false)]");
        let error = parse_serde_field_attrs(&attrs, &ident, context)
            .expect_err("serde default non-string values should be rejected");
        assert!(error.to_string().contains("expected string literal"));
    }

    #[test]
    fn field_naming_probe_uses_default_syntax_rules() {
        let (_, attrs) = field_attrs("#[serde(default = \"default_field\")]");
        assert!(super::has_field_naming_attrs(&attrs).expect("valid default path parses"));

        let (_, attrs) = field_attrs("#[serde(default = false)]");
        let error = super::has_field_naming_attrs(&attrs)
            .expect_err("invalid default value should be rejected");
        assert!(error.to_string().contains("expected string literal"));
    }
}
