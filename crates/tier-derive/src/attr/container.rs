use syn::Attribute;

use super::model::{
    ContainerPathListSpec, ContainerPathSpec, ContainerValidationCheck, TierContainerAttrs,
};
use crate::syntax::{
    parse_expr_list_call, parse_expr_value, parse_string_list_call, parse_string_value,
    parse_value_expr,
};

pub(crate) fn parse_tier_container_attrs(
    attributes: &[Attribute],
) -> syn::Result<TierContainerAttrs> {
    let mut attrs = TierContainerAttrs::default();

    for attribute in attributes {
        if !attribute.path().is_ident("tier") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("at_least_one_of") {
                attrs.checks.push(ContainerValidationCheck::AtLeastOneOf(
                    ContainerPathListSpec::Strings(parse_string_list_call(meta)?),
                ));
                return Ok(());
            }
            if meta.path.is_ident("at_least_one_of_expr") {
                attrs.checks.push(ContainerValidationCheck::AtLeastOneOf(
                    ContainerPathListSpec::Exprs(parse_expr_list_call(meta)?),
                ));
                return Ok(());
            }
            if meta.path.is_ident("exactly_one_of") {
                attrs.checks.push(ContainerValidationCheck::ExactlyOneOf(
                    ContainerPathListSpec::Strings(parse_string_list_call(meta)?),
                ));
                return Ok(());
            }
            if meta.path.is_ident("exactly_one_of_expr") {
                attrs.checks.push(ContainerValidationCheck::ExactlyOneOf(
                    ContainerPathListSpec::Exprs(parse_expr_list_call(meta)?),
                ));
                return Ok(());
            }
            if meta.path.is_ident("mutually_exclusive") {
                attrs
                    .checks
                    .push(ContainerValidationCheck::MutuallyExclusive(
                        ContainerPathListSpec::Strings(parse_string_list_call(meta)?),
                    ));
                return Ok(());
            }
            if meta.path.is_ident("mutually_exclusive_expr") {
                attrs
                    .checks
                    .push(ContainerValidationCheck::MutuallyExclusive(
                        ContainerPathListSpec::Exprs(parse_expr_list_call(meta)?),
                    ));
                return Ok(());
            }
            if meta.path.is_ident("required_with") {
                attrs
                    .checks
                    .push(parse_required_with_container_check(meta)?);
                return Ok(());
            }
            if meta.path.is_ident("required_if") {
                attrs.checks.push(parse_required_if_container_check(meta)?);
                return Ok(());
            }
            Err(meta.error("unsupported tier container attribute"))
        })?;
    }

    Ok(attrs)
}

fn parse_required_with_container_check(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<ContainerValidationCheck> {
    let mut path = None::<ContainerPathSpec>;
    let mut requires = None::<ContainerPathListSpec>;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("path") {
            if path.is_some() {
                return Err(nested.error(
                    "required_with supports either `path = ...` or `path_expr = ...`, not both",
                ));
            }
            path = Some(ContainerPathSpec::String(parse_string_value(nested)?));
            return Ok(());
        }
        if nested.path.is_ident("path_expr") {
            if path.is_some() {
                return Err(nested.error(
                    "required_with supports either `path = ...` or `path_expr = ...`, not both",
                ));
            }
            path = Some(ContainerPathSpec::Expr(parse_expr_value(nested)?));
            return Ok(());
        }
        if nested.path.is_ident("requires") {
            if requires.is_some() {
                return Err(nested.error(
                    "required_with supports either `requires(...)` or `requires_expr(...)`, not both",
                ));
            }
            requires = Some(ContainerPathListSpec::Strings(parse_string_list_call(nested)?));
            return Ok(());
        }
        if nested.path.is_ident("requires_expr") {
            if requires.is_some() {
                return Err(nested.error(
                    "required_with supports either `requires(...)` or `requires_expr(...)`, not both",
                ));
            }
            requires = Some(ContainerPathListSpec::Exprs(parse_expr_list_call(nested)?));
            return Ok(());
        }
        Err(nested.error("unsupported required_with option"))
    })?;

    let Some(path) = path else {
        return Err(meta.error("required_with requires `path = \"...\"` or `path_expr = ...`"));
    };
    let Some(requires) = requires else {
        return Err(
            meta.error("required_with requires `requires(\"...\")` or `requires_expr(...)`")
        );
    };

    Ok(ContainerValidationCheck::RequiredWith { path, requires })
}

fn parse_required_if_container_check(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<ContainerValidationCheck> {
    let mut path = None::<ContainerPathSpec>;
    let mut equals = None;
    let mut requires = None::<ContainerPathListSpec>;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("path") {
            if path.is_some() {
                return Err(nested.error(
                    "required_if supports either `path = ...` or `path_expr = ...`, not both",
                ));
            }
            path = Some(ContainerPathSpec::String(parse_string_value(nested)?));
            return Ok(());
        }
        if nested.path.is_ident("path_expr") {
            if path.is_some() {
                return Err(nested.error(
                    "required_if supports either `path = ...` or `path_expr = ...`, not both",
                ));
            }
            path = Some(ContainerPathSpec::Expr(parse_expr_value(nested)?));
            return Ok(());
        }
        if nested.path.is_ident("equals") {
            if equals.is_some() {
                return Err(nested.error("required_if supports only one `equals = ...` option"));
            }
            equals = Some(parse_value_expr(nested)?);
            return Ok(());
        }
        if nested.path.is_ident("requires") {
            if requires.is_some() {
                return Err(nested.error(
                    "required_if supports either `requires(...)` or `requires_expr(...)`, not both",
                ));
            }
            requires = Some(ContainerPathListSpec::Strings(parse_string_list_call(
                nested,
            )?));
            return Ok(());
        }
        if nested.path.is_ident("requires_expr") {
            if requires.is_some() {
                return Err(nested.error(
                    "required_if supports either `requires(...)` or `requires_expr(...)`, not both",
                ));
            }
            requires = Some(ContainerPathListSpec::Exprs(parse_expr_list_call(nested)?));
            return Ok(());
        }
        Err(nested.error("unsupported required_if option"))
    })?;

    let Some(path) = path else {
        return Err(meta.error("required_if requires `path = \"...\"` or `path_expr = ...`"));
    };
    let Some(equals) = equals else {
        return Err(meta.error("required_if requires `equals = ...`"));
    };
    let Some(requires) = requires else {
        return Err(meta.error("required_if requires `requires(\"...\")` or `requires_expr(...)`"));
    };

    Ok(ContainerValidationCheck::RequiredIf {
        path,
        equals,
        requires,
    })
}

#[cfg(test)]
mod tests {
    use syn::{Attribute, Data, DeriveInput};

    use super::parse_tier_container_attrs;

    fn container_attrs(attribute: &str) -> Vec<Attribute> {
        let input = syn::parse_str::<DeriveInput>(&format!(
            "{attribute} struct Config {{ field: String }}"
        ))
        .expect("test struct parses");
        let Data::Struct(_) = input.data else {
            unreachable!("test input is a struct");
        };
        input.attrs
    }

    #[test]
    fn required_if_rejects_duplicate_equals_options() {
        let error = parse_tier_container_attrs(&container_attrs(
            "#[tier(required_if(path = \"tls.enabled\", equals = true, equals = false, requires(\"tls.cert\")))]",
        ))
        .expect_err("duplicate equals options should be rejected");

        assert!(
            error
                .to_string()
                .contains("required_if supports only one `equals = ...` option")
        );
    }
}
