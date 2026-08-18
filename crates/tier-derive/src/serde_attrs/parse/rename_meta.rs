use syn::LitStr;

use crate::serde_attrs::rename::RenameRule;
use crate::syntax::{parse_string_value, reject_duplicate_option};

pub(super) fn parse_rename_all_meta(
    meta: syn::meta::ParseNestedMeta<'_>,
    serialize: &mut Option<RenameRule>,
    deserialize: &mut Option<RenameRule>,
    attribute: &str,
) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        reject_duplicate_option(serialize, &meta, &format!("serde attribute `{attribute}`"))?;
        reject_duplicate_option(
            deserialize,
            &meta,
            &format!("serde attribute `{attribute}`"),
        )?;
        let literal: LitStr = meta.value()?.parse()?;
        let rule = RenameRule::parse(&literal.value(), literal.span())?;
        *serialize = Some(rule);
        *deserialize = Some(rule);
        return Ok(());
    }

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("serialize") {
            reject_duplicate_option(
                serialize,
                &nested,
                &format!("serde attribute `{attribute}(serialize = ...)`"),
            )?;
            let literal: LitStr = nested.value()?.parse()?;
            *serialize = Some(RenameRule::parse(&literal.value(), literal.span())?);
            return Ok(());
        }
        if nested.path.is_ident("deserialize") {
            reject_duplicate_option(
                deserialize,
                &nested,
                &format!("serde attribute `{attribute}(deserialize = ...)`"),
            )?;
            let literal: LitStr = nested.value()?.parse()?;
            *deserialize = Some(RenameRule::parse(&literal.value(), literal.span())?);
            return Ok(());
        }
        Err(nested.error(format!("unsupported serde {attribute} option")))
    })
}

pub(super) fn parse_rename_meta(
    meta: syn::meta::ParseNestedMeta<'_>,
    serialize: &mut Option<String>,
    deserialize: &mut Option<String>,
) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        reject_duplicate_option(serialize, &meta, "serde attribute `rename`")?;
        reject_duplicate_option(deserialize, &meta, "serde attribute `rename`")?;
        let value = parse_string_value(meta)?;
        *serialize = Some(value.clone());
        *deserialize = Some(value);
        return Ok(());
    }

    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("serialize") {
            reject_duplicate_option(
                serialize,
                &nested,
                "serde attribute `rename(serialize = ...)`",
            )?;
            *serialize = Some(parse_string_value(nested)?);
            return Ok(());
        }
        if nested.path.is_ident("deserialize") {
            reject_duplicate_option(
                deserialize,
                &nested,
                "serde attribute `rename(deserialize = ...)`",
            )?;
            *deserialize = Some(parse_string_value(nested)?);
            return Ok(());
        }
        Err(nested.error("unsupported serde rename option"))
    })
}
