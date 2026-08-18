use syn::{Attribute, spanned::Spanned};

use super::model::{TierAttrs, TierSourceKind};
use crate::syntax::{
    parse_flag, parse_literal_expr_list, parse_numeric_literal, parse_rule_key_string_list,
    parse_rule_key_string_value, parse_string_list_call, parse_string_value, parse_usize_value,
    reject_duplicate_flag, reject_duplicate_option, reject_duplicate_slice,
};

pub(crate) fn parse_tier_attrs(attributes: &[Attribute]) -> syn::Result<TierAttrs> {
    let mut attrs = TierAttrs::default();
    for attribute in attributes {
        if !attribute.path().is_ident("tier") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("secret") {
                reject_duplicate_flag(attrs.secret, &meta, "tier attribute `secret`")?;
                parse_flag(meta)?;
                attrs.secret = true;
                return Ok(());
            }
            if meta.path.is_ident("leaf") {
                reject_duplicate_flag(attrs.leaf, &meta, "tier attribute `leaf`")?;
                parse_flag(meta)?;
                attrs.leaf = true;
                return Ok(());
            }
            if meta.path.is_ident("sources") {
                reject_duplicate_slice(&attrs.sources, &meta, "tier attribute `sources`")?;
                attrs.sources = parse_source_kind_list(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("deny_sources") {
                reject_duplicate_slice(
                    &attrs.deny_sources,
                    &meta,
                    "tier attribute `deny_sources`",
                )?;
                attrs.deny_sources = parse_source_kind_list(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("env") {
                reject_duplicate_option(&attrs.env, &meta, "tier attribute `env`")?;
                attrs.env = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("doc") {
                reject_duplicate_option(&attrs.doc, &meta, "tier attribute `doc`")?;
                attrs.doc = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("example") {
                reject_duplicate_option(&attrs.example, &meta, "tier attribute `example`")?;
                attrs.example = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("deprecated") {
                reject_duplicate_option(&attrs.deprecated, &meta, "tier attribute `deprecated`")?;
                attrs.deprecated = Some(if meta.input.peek(syn::Token![=]) {
                    parse_string_value(meta)?
                } else {
                    "this field is deprecated".to_owned()
                });
                return Ok(());
            }
            if meta.path.is_ident("merge") {
                reject_duplicate_option(&attrs.merge, &meta, "tier attribute `merge`")?;
                attrs.merge = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("non_empty") {
                reject_duplicate_flag(attrs.non_empty, &meta, "tier attribute `non_empty`")?;
                parse_flag(meta)?;
                attrs.non_empty = true;
                return Ok(());
            }
            if meta.path.is_ident("min") {
                reject_duplicate_option(&attrs.min, &meta, "tier attribute `min`")?;
                attrs.min = Some(parse_numeric_literal(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("max") {
                reject_duplicate_option(&attrs.max, &meta, "tier attribute `max`")?;
                attrs.max = Some(parse_numeric_literal(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("min_length") {
                reject_duplicate_option(&attrs.min_length, &meta, "tier attribute `min_length`")?;
                attrs.min_length = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("max_length") {
                reject_duplicate_option(&attrs.max_length, &meta, "tier attribute `max_length`")?;
                attrs.max_length = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("min_items") {
                reject_duplicate_option(&attrs.min_items, &meta, "tier attribute `min_items`")?;
                attrs.min_items = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("max_items") {
                reject_duplicate_option(&attrs.max_items, &meta, "tier attribute `max_items`")?;
                attrs.max_items = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("min_properties") {
                reject_duplicate_option(
                    &attrs.min_properties,
                    &meta,
                    "tier attribute `min_properties`",
                )?;
                attrs.min_properties = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("max_properties") {
                reject_duplicate_option(
                    &attrs.max_properties,
                    &meta,
                    "tier attribute `max_properties`",
                )?;
                attrs.max_properties = Some(parse_usize_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("multiple_of") {
                reject_duplicate_option(&attrs.multiple_of, &meta, "tier attribute `multiple_of`")?;
                attrs.multiple_of = Some(parse_numeric_literal(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("pattern") {
                reject_duplicate_option(&attrs.pattern, &meta, "tier attribute `pattern`")?;
                attrs.pattern = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("unique_items") {
                reject_duplicate_flag(attrs.unique_items, &meta, "tier attribute `unique_items`")?;
                parse_flag(meta)?;
                attrs.unique_items = true;
                return Ok(());
            }
            if meta.path.is_ident("one_of") {
                reject_duplicate_slice(&attrs.one_of, &meta, "tier attribute `one_of`")?;
                attrs.one_of = parse_literal_expr_list(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("hostname") {
                reject_duplicate_flag(attrs.hostname, &meta, "tier attribute `hostname`")?;
                parse_flag(meta)?;
                attrs.hostname = true;
                return Ok(());
            }
            if meta.path.is_ident("url") {
                reject_duplicate_flag(attrs.url, &meta, "tier attribute `url`")?;
                parse_flag(meta)?;
                attrs.url = true;
                return Ok(());
            }
            if meta.path.is_ident("email") {
                reject_duplicate_flag(attrs.email, &meta, "tier attribute `email`")?;
                parse_flag(meta)?;
                attrs.email = true;
                return Ok(());
            }
            if meta.path.is_ident("ip_addr") {
                reject_duplicate_flag(attrs.ip_addr, &meta, "tier attribute `ip_addr`")?;
                parse_flag(meta)?;
                attrs.ip_addr = true;
                return Ok(());
            }
            if meta.path.is_ident("socket_addr") {
                reject_duplicate_flag(attrs.socket_addr, &meta, "tier attribute `socket_addr`")?;
                parse_flag(meta)?;
                attrs.socket_addr = true;
                return Ok(());
            }
            if meta.path.is_ident("absolute_path") {
                reject_duplicate_flag(
                    attrs.absolute_path,
                    &meta,
                    "tier attribute `absolute_path`",
                )?;
                parse_flag(meta)?;
                attrs.absolute_path = true;
                return Ok(());
            }
            if meta.path.is_ident("env_decode") {
                reject_duplicate_option(&attrs.env_decode, &meta, "tier attribute `env_decode`")?;
                attrs.env_decode = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("validation_message") {
                let span = meta.path.span();
                let (rule, value) = parse_rule_key_string_value(meta)?;
                reject_duplicate_rule_config(
                    &attrs.validation_messages,
                    &rule,
                    "validation_message",
                    span,
                )?;
                attrs.validation_messages.push((rule, value));
                return Ok(());
            }
            if meta.path.is_ident("validation_level") {
                let span = meta.path.span();
                let (rule, value) = parse_rule_key_string_value(meta)?;
                reject_duplicate_rule_config(
                    &attrs.validation_levels,
                    &rule,
                    "validation_level",
                    span,
                )?;
                attrs.validation_levels.push((rule, value));
                return Ok(());
            }
            if meta.path.is_ident("validation_tags") {
                let span = meta.path.span();
                let (rule, values) = parse_rule_key_string_list(meta)?;
                reject_duplicate_rule_config(
                    &attrs.validation_tags,
                    &rule,
                    "validation_tags",
                    span,
                )?;
                attrs.validation_tags.push((rule, values));
                return Ok(());
            }
            Err(meta.error("unsupported tier attribute"))
        })?;
    }
    Ok(attrs)
}

fn parse_source_kind_list(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<Vec<TierSourceKind>> {
    let span = meta.path.span();
    parse_string_list_call(meta)?
        .into_iter()
        .map(|value| TierSourceKind::parse(&value, span))
        .collect()
}

fn reject_duplicate_rule_config<T>(
    configs: &[(String, T)],
    rule: &str,
    attribute: &str,
    span: proc_macro2::Span,
) -> syn::Result<()> {
    if configs
        .iter()
        .any(|(existing_rule, _)| existing_rule == rule)
    {
        return Err(syn::Error::new(
            span,
            format!("duplicate tier attribute `{attribute}` for validation rule `{rule}`"),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, Data, DeriveInput, Fields};

    use super::parse_tier_attrs;

    fn field_attrs(attribute: &str) -> Vec<Attribute> {
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
        fields
            .named
            .into_iter()
            .next()
            .expect("test input has a field")
            .attrs
    }

    #[test]
    fn flag_attrs_accept_bare_markers() {
        let attrs = parse_tier_attrs(&field_attrs(
            "#[tier(secret, leaf, non_empty, unique_items, hostname, url, email, ip_addr, socket_addr, absolute_path)]",
        ))
        .expect("bare flag attributes parse");

        assert!(attrs.secret);
        assert!(attrs.leaf);
        assert!(attrs.non_empty);
        assert!(attrs.unique_items);
        assert!(attrs.hostname);
        assert!(attrs.url);
        assert!(attrs.email);
        assert!(attrs.ip_addr);
        assert!(attrs.socket_addr);
        assert!(attrs.absolute_path);
    }

    #[test]
    fn flag_attrs_reject_values() {
        let error = parse_tier_attrs(&field_attrs("#[tier(non_empty = false)]"))
            .expect_err("flag values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn flag_attrs_reject_arguments() {
        let error = parse_tier_attrs(&field_attrs("#[tier(url(\"https://example.com\"))]"))
            .expect_err("flag arguments should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take arguments")
        );
    }

    #[test]
    fn singular_attrs_reject_duplicates() {
        let error = parse_tier_attrs(&field_attrs("#[tier(env = \"APP_A\", env = \"APP_B\")]"))
            .expect_err("duplicate singular attributes should be rejected");

        assert!(error.to_string().contains("duplicate tier attribute `env`"));
    }

    #[test]
    fn flag_attrs_reject_duplicates() {
        let error = parse_tier_attrs(&field_attrs("#[tier(secret, secret)]"))
            .expect_err("duplicate flag attributes should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate tier attribute `secret`")
        );
    }

    #[test]
    fn list_attrs_reject_duplicates() {
        let error = parse_tier_attrs(&field_attrs("#[tier(one_of(\"a\"), one_of(\"b\"))]"))
            .expect_err("duplicate list attributes should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate tier attribute `one_of`")
        );
    }

    #[test]
    fn validation_config_attrs_reject_duplicate_rules() {
        let error = parse_tier_attrs(&field_attrs(
            "#[tier(validation_level(rule = \"url\", value = \"warning\"), validation_level(rule = \"url\", value = \"error\"))]",
        ))
        .expect_err("duplicate validation config rules should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate tier attribute `validation_level` for validation rule `url`")
        );
    }

    #[test]
    fn validation_config_attrs_reject_duplicate_nested_options() {
        let error = parse_tier_attrs(&field_attrs(
            "#[tier(validation_message(rule = \"url\", rule = \"email\", value = \"invalid\"))]",
        ))
        .expect_err("duplicate nested rule options should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate validation rule config option `rule`")
        );

        let error = parse_tier_attrs(&field_attrs(
            "#[tier(validation_level(rule = \"url\", value = \"warning\", value = \"error\"))]",
        ))
        .expect_err("duplicate nested value options should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate validation rule config option `value`")
        );

        let error = parse_tier_attrs(&field_attrs(
            "#[tier(validation_tags(rule = \"url\", values(\"network\"), values(\"external\")))]",
        ))
        .expect_err("duplicate nested values options should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate validation tag config option `values`")
        );
    }
}
