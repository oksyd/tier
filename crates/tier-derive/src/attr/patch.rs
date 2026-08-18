use syn::Attribute;

use super::model::PatchAttrs;
use crate::syntax::{
    parse_expr_value, parse_flag, parse_string_value, reject_duplicate_flag,
    reject_duplicate_option,
};

pub(crate) fn parse_patch_attrs(attributes: &[Attribute]) -> syn::Result<PatchAttrs> {
    let mut attrs = PatchAttrs::default();
    for attribute in attributes {
        if !attribute.path().is_ident("tier") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("path") {
                reject_duplicate_option(&attrs.path, &meta, "tier patch attribute `path`")?;
                attrs.path = Some(parse_string_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("path_expr") {
                reject_duplicate_option(
                    &attrs.path_expr,
                    &meta,
                    "tier patch attribute `path_expr`",
                )?;
                attrs.path_expr = Some(parse_expr_value(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("nested") {
                reject_duplicate_flag(attrs.nested, &meta, "tier patch attribute `nested`")?;
                parse_flag(meta)?;
                attrs.nested = true;
                return Ok(());
            }
            if meta.path.is_ident("skip") {
                reject_duplicate_flag(attrs.skip, &meta, "tier patch attribute `skip`")?;
                parse_flag(meta)?;
                attrs.skip = true;
                return Ok(());
            }
            Err(meta.error("unsupported tier patch attribute"))
        })?;
    }
    Ok(attrs)
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, Data, DeriveInput, Fields};

    use super::parse_patch_attrs;

    fn field_attrs(attribute: &str) -> Vec<Attribute> {
        let input = syn::parse_str::<DeriveInput>(&format!(
            "struct Patch {{ {attribute} field: Option<String> }}"
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
        let attrs = parse_patch_attrs(&field_attrs("#[tier(nested, skip)]"))
            .expect("bare patch flag attributes parse");

        assert!(attrs.nested);
        assert!(attrs.skip);
    }

    #[test]
    fn flag_attrs_reject_values() {
        let error = parse_patch_attrs(&field_attrs("#[tier(skip = false)]"))
            .expect_err("patch flag values should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take a value")
        );
    }

    #[test]
    fn flag_attrs_reject_arguments() {
        let error = parse_patch_attrs(&field_attrs("#[tier(nested(\"db\"))]"))
            .expect_err("patch flag arguments should be rejected");

        assert!(
            error
                .to_string()
                .contains("flag attribute does not take arguments")
        );
    }

    #[test]
    fn singular_attrs_reject_duplicates() {
        let error = parse_patch_attrs(&field_attrs("#[tier(path = \"db.a\", path = \"db.b\")]"))
            .expect_err("duplicate patch path attributes should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate tier patch attribute `path`")
        );
    }

    #[test]
    fn flag_attrs_reject_duplicates() {
        let error = parse_patch_attrs(&field_attrs("#[tier(skip, skip)]"))
            .expect_err("duplicate patch flag attributes should be rejected");

        assert!(
            error
                .to_string()
                .contains("duplicate tier patch attribute `skip`")
        );
    }
}
