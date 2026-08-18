use syn::Attribute;

use crate::serde_attrs::model::SerdeContainerAttrs;
use crate::syntax::{
    consume_unused_meta, parse_bare_or_string_value, parse_flag, parse_string_value,
    reject_duplicate_flag, reject_duplicate_option,
};

use super::rename_meta::parse_rename_all_meta;

pub(crate) fn parse_serde_container_attrs(
    attributes: &[Attribute],
) -> syn::Result<SerdeContainerAttrs> {
    let mut attrs = SerdeContainerAttrs::default();
    for attribute in attributes {
        if !attribute.path().is_ident("serde") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                parse_rename_all_meta(
                    meta,
                    &mut attrs.rename_all_serialize,
                    &mut attrs.rename_all_deserialize,
                    "rename_all",
                )?;
                return Ok(());
            }
            if meta.path.is_ident("rename_all_fields") {
                parse_rename_all_meta(
                    meta,
                    &mut attrs.rename_all_fields_serialize,
                    &mut attrs.rename_all_fields_deserialize,
                    "rename_all_fields",
                )?;
                return Ok(());
            }
            if meta.path.is_ident("default") {
                reject_duplicate_flag(attrs.default_fields, &meta, "serde attribute `default`")?;
                attrs.default_fields = true;
                parse_bare_or_string_value(meta)?;
                return Ok(());
            }
            if meta.path.is_ident("tag") {
                reject_duplicate_option(&attrs.tag, &meta, "serde attribute `tag`")?;
                attrs.tag = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("content") {
                reject_duplicate_option(&attrs.content, &meta, "serde attribute `content`")?;
                attrs.content = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("untagged") {
                reject_duplicate_flag(attrs.untagged, &meta, "serde attribute `untagged`")?;
                parse_flag(meta)?;
                attrs.untagged = true;
                return Ok(());
            }
            consume_unused_meta(meta)?;
            Ok(())
        })?;
    }

    Ok(attrs)
}

#[cfg(test)]
mod tests {
    use syn::DeriveInput;

    use super::parse_serde_container_attrs;

    #[test]
    fn serde_container_flags_reject_values_when_they_change_tier_metadata() {
        let input =
            syn::parse_str::<DeriveInput>("#[serde(untagged = false)] enum Config { Variant }")
                .expect("test enum parses");
        let error = parse_serde_container_attrs(&input.attrs)
            .expect_err("serde untagged values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn serde_container_rejects_duplicate_rename_all() {
        let input = syn::parse_str::<DeriveInput>(
            "#[serde(rename_all = \"snake_case\", rename_all = \"kebab-case\")] struct Config { field: String }",
        )
        .expect("test struct parses");
        let error = parse_serde_container_attrs(&input.attrs)
            .expect_err("duplicate serde rename_all should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `rename_all`")
        );
    }

    #[test]
    fn serde_container_rejects_duplicate_rename_all_fields_with_precise_label() {
        let input = syn::parse_str::<DeriveInput>(
            "#[serde(rename_all_fields = \"snake_case\", rename_all_fields = \"kebab-case\")] enum Config { Variant { field: String } }",
        )
        .expect("test enum parses");
        let error = parse_serde_container_attrs(&input.attrs)
            .expect_err("duplicate serde rename_all_fields should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `rename_all_fields`")
        );
    }

    #[test]
    fn serde_container_rejects_duplicate_tag_content_and_flags() {
        let tagged = syn::parse_str::<DeriveInput>(
            "#[serde(tag = \"type\", tag = \"kind\")] enum Config { Variant }",
        )
        .expect("test enum parses");
        let error = parse_serde_container_attrs(&tagged.attrs)
            .expect_err("duplicate serde tag should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `tag`")
        );

        let adjacent = syn::parse_str::<DeriveInput>(
            "#[serde(content = \"value\", content = \"data\")] enum Config { Variant }",
        )
        .expect("test enum parses");
        let error = parse_serde_container_attrs(&adjacent.attrs)
            .expect_err("duplicate serde content should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `content`")
        );

        let untagged =
            syn::parse_str::<DeriveInput>("#[serde(untagged, untagged)] enum Config { Variant }")
                .expect("test enum parses");
        let error = parse_serde_container_attrs(&untagged.attrs)
            .expect_err("duplicate serde untagged should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `untagged`")
        );

        let defaulted = syn::parse_str::<DeriveInput>(
            "#[serde(default, default)] struct Config { field: String }",
        )
        .expect("test struct parses");
        let error = parse_serde_container_attrs(&defaulted.attrs)
            .expect_err("duplicate serde default should be rejected");
        assert!(
            error
                .to_string()
                .contains("duplicate serde attribute `default`")
        );
    }

    #[test]
    fn serde_container_default_accepts_only_bare_or_string_path_forms() {
        let input = syn::parse_str::<DeriveInput>(
            "#[serde(default = \"default_config\")] struct Config { field: String }",
        )
        .expect("test struct parses");
        parse_serde_container_attrs(&input.attrs).expect("serde default string path should parse");

        let input = syn::parse_str::<DeriveInput>(
            "#[serde(default = false)] struct Config { field: String }",
        )
        .expect("test struct parses");
        let error = parse_serde_container_attrs(&input.attrs)
            .expect_err("serde default non-string values should be rejected");
        assert!(error.to_string().contains("expected string literal"));
    }
}
