use quote::quote;
use syn::{Expr, Lit, LitStr, punctuated::Punctuated, spanned::Spanned};

use super::attr::{NumericLiteral, NumericLiteralValue};

pub(crate) fn parse_expr_value(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<Expr> {
    meta.value()?.parse()
}

pub(crate) fn parse_string_value(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<String> {
    let literal: LitStr = meta.value()?.parse()?;
    Ok(literal.value())
}

pub(crate) fn parse_usize_value(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<usize> {
    let literal: syn::LitInt = meta.value()?.parse()?;
    literal.base10_parse()
}

pub(crate) fn parse_string_list_call(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<Vec<String>> {
    let content;
    syn::parenthesized!(content in meta.input);
    let values = Punctuated::<LitStr, syn::Token![,]>::parse_terminated(&content)?;
    if values.is_empty() {
        return Err(meta.error("expected at least one string literal"));
    }
    Ok(values.into_iter().map(|value| value.value()).collect())
}

pub(crate) fn parse_expr_list_call(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<Vec<Expr>> {
    let content;
    syn::parenthesized!(content in meta.input);
    let values = Punctuated::<Expr, syn::Token![,]>::parse_terminated(&content)?;
    if values.is_empty() {
        return Err(meta.error("expected at least one expression"));
    }
    Ok(values.into_iter().collect())
}

pub(crate) fn parse_literal_expr_list(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<Vec<Expr>> {
    let content;
    syn::parenthesized!(content in meta.input);
    let values = Punctuated::<Expr, syn::Token![,]>::parse_terminated(&content)?;
    if values.is_empty() {
        return Err(meta.error("expected at least one literal value"));
    }
    let values = values.into_iter().collect::<Vec<_>>();
    for value in &values {
        validate_value_expr(value, value.span())?;
    }
    Ok(values)
}

pub(crate) fn parse_rule_key_string_value(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<(String, String)> {
    let mut rule = None::<String>;
    let mut value = None::<String>;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("rule") {
            reject_duplicate_option(&rule, &nested, "validation rule config option `rule`")?;
            rule = Some(parse_string_value(nested)?);
            return Ok(());
        }
        if nested.path.is_ident("value") {
            reject_duplicate_option(&value, &nested, "validation rule config option `value`")?;
            value = Some(parse_string_value(nested)?);
            return Ok(());
        }
        Err(nested.error("unsupported validation rule config option"))
    })?;

    let Some(rule) = rule else {
        return Err(meta.error("validation rule config requires `rule = \"...\"`"));
    };
    let Some(value) = value else {
        return Err(meta.error("validation rule config requires `value = \"...\"`"));
    };
    Ok((rule, value))
}

pub(crate) fn parse_rule_key_string_list(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<(String, Vec<String>)> {
    let mut rule = None::<String>;
    let mut values = None::<Vec<String>>;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("rule") {
            reject_duplicate_option(&rule, &nested, "validation tag config option `rule`")?;
            rule = Some(parse_string_value(nested)?);
            return Ok(());
        }
        if nested.path.is_ident("values") {
            reject_duplicate_option(&values, &nested, "validation tag config option `values`")?;
            values = Some(parse_string_list_call(nested)?);
            return Ok(());
        }
        Err(nested.error("unsupported validation tag config option"))
    })?;

    let Some(rule) = rule else {
        return Err(meta.error("validation tag config requires `rule = \"...\"`"));
    };
    let Some(values) = values else {
        return Err(meta.error("validation tag config requires `values(\"...\")`"));
    };
    Ok((rule, values))
}

pub(crate) fn parse_numeric_literal(
    meta: syn::meta::ParseNestedMeta<'_>,
) -> syn::Result<NumericLiteral> {
    let expr: Expr = meta.value()?.parse()?;
    parse_numeric_expr(expr, meta.path.span())
}

fn parse_numeric_expr(expr: Expr, span: proc_macro2::Span) -> syn::Result<NumericLiteral> {
    match expr {
        Expr::Lit(expr_lit) => match expr_lit.lit {
            Lit::Int(literal) => Ok(NumericLiteral {
                tokens: quote! { #literal },
                value: parse_decimal_numeric_literal(literal.base10_digits(), false, span)?,
            }),
            Lit::Float(literal) => Ok(NumericLiteral {
                tokens: quote! { #literal },
                value: parse_decimal_numeric_literal(literal.base10_digits(), false, span)?,
            }),
            _ => Err(syn::Error::new(
                span,
                "expected an integer or float literal",
            )),
        },
        Expr::Unary(expr_unary) if matches!(expr_unary.op, syn::UnOp::Neg(_)) => {
            match *expr_unary.expr {
                Expr::Lit(expr_lit) => match expr_lit.lit {
                    Lit::Int(literal) => Ok(NumericLiteral {
                        tokens: quote! { -#literal },
                        value: parse_decimal_numeric_literal(literal.base10_digits(), true, span)?,
                    }),
                    Lit::Float(literal) => Ok(NumericLiteral {
                        tokens: quote! { -#literal },
                        value: parse_decimal_numeric_literal(literal.base10_digits(), true, span)?,
                    }),
                    _ => Err(syn::Error::new(
                        span,
                        "expected an integer or float literal",
                    )),
                },
                _ => Err(syn::Error::new(
                    span,
                    "expected an integer or float literal",
                )),
            }
        }
        _ => Err(syn::Error::new(
            span,
            "expected an integer or float literal",
        )),
    }
}

fn parse_decimal_numeric_literal(
    text: &str,
    negative: bool,
    span: proc_macro2::Span,
) -> syn::Result<NumericLiteralValue> {
    let text = text.replace('_', "");
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (
            mantissa,
            exponent
                .parse::<i32>()
                .map_err(|_| numeric_literal_error(span))?,
        ),
        None => (text.as_str(), 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let numerator = digits
        .parse::<u128>()
        .map_err(|_| numeric_literal_error(span))?;
    let fraction_digits = i32::try_from(fraction.len()).map_err(|_| numeric_literal_error(span))?;
    let scale = fraction_digits
        .checked_sub(exponent)
        .ok_or_else(|| numeric_literal_error(span))?;

    let (numerator, denominator) = if scale <= 0 {
        let multiplier =
            pow10_u128(scale.unsigned_abs()).ok_or_else(|| numeric_literal_error(span))?;
        (
            numerator
                .checked_mul(multiplier)
                .ok_or_else(|| numeric_literal_error(span))?,
            1,
        )
    } else {
        (
            numerator,
            pow10_u128(u32::try_from(scale).map_err(|_| numeric_literal_error(span))?)
                .ok_or_else(|| numeric_literal_error(span))?,
        )
    };

    Ok(NumericLiteralValue::new(negative, numerator, denominator))
}

fn pow10_u128(exponent: u32) -> Option<u128> {
    let mut value = 1u128;
    for _ in 0..exponent {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

fn numeric_literal_error(span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(
        span,
        "numeric literal is too large for exact tier validation",
    )
}

pub(crate) fn parse_value_expr(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<Expr> {
    let expr: Expr = meta.value()?.parse()?;
    validate_value_expr(&expr, meta.path.span())?;
    Ok(expr)
}

pub(crate) fn parse_flag(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        return Err(meta.error("flag attribute does not take a value"));
    }

    if meta.input.peek(syn::token::Paren) {
        return Err(meta.error("flag attribute does not take arguments"));
    }

    Ok(())
}

pub(crate) fn parse_bare_or_string_value(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let _: LitStr = meta.value()?.parse()?;
        return Ok(());
    }

    if meta.input.peek(syn::token::Paren) {
        return Err(meta.error("attribute does not take arguments"));
    }

    Ok(())
}

pub(crate) fn reject_duplicate_flag(
    enabled: bool,
    meta: &syn::meta::ParseNestedMeta<'_>,
    label: &str,
) -> syn::Result<()> {
    if enabled {
        return Err(meta.error(format!("duplicate {label}")));
    }

    Ok(())
}

pub(crate) fn reject_duplicate_option<T>(
    slot: &Option<T>,
    meta: &syn::meta::ParseNestedMeta<'_>,
    label: &str,
) -> syn::Result<()> {
    if slot.is_some() {
        return Err(meta.error(format!("duplicate {label}")));
    }

    Ok(())
}

pub(crate) fn reject_duplicate_slice<T>(
    values: &[T],
    meta: &syn::meta::ParseNestedMeta<'_>,
    label: &str,
) -> syn::Result<()> {
    if !values.is_empty() {
        return Err(meta.error(format!("duplicate {label}")));
    }

    Ok(())
}

fn validate_value_expr(expr: &Expr, span: proc_macro2::Span) -> syn::Result<()> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Str(_) | Lit::Bool(_) | Lit::Int(_) | Lit::Float(_) => Ok(()),
            _ => Err(syn::Error::new(
                span,
                "expected a string, bool, integer, or float literal",
            )),
        },
        Expr::Unary(expr_unary) if matches!(expr_unary.op, syn::UnOp::Neg(_)) => match &*expr_unary
            .expr
        {
            Expr::Lit(expr_lit) if matches!(expr_lit.lit, Lit::Int(_) | Lit::Float(_)) => Ok(()),
            _ => Err(syn::Error::new(
                span,
                "expected a string, bool, integer, or float literal",
            )),
        },
        _ => Err(syn::Error::new(
            span,
            "expected a string, bool, integer, or float literal",
        )),
    }
}

pub(crate) fn unraw(ident: &syn::Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

pub(crate) fn consume_unused_meta(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let _: Expr = meta.value()?.parse()?;
        return Ok(());
    }

    if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|nested| {
            consume_unused_meta(nested)?;
            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use syn::Expr;

    use super::parse_numeric_expr;

    fn parse(value: &str) -> super::NumericLiteralValue {
        let expr = syn::parse_str::<Expr>(value).expect("valid numeric expression");
        parse_numeric_expr(expr, proc_macro2::Span::call_site())
            .expect("numeric literal parses")
            .value
    }

    #[test]
    fn numeric_literal_comparison_preserves_large_integer_precision() {
        assert_eq!(
            parse("9007199254740993").cmp_exact(&parse("9007199254740992")),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn numeric_literal_comparison_preserves_decimal_precision() {
        assert_eq!(
            parse("0.10").cmp_exact(&parse("0.1")),
            Some(Ordering::Equal)
        );
        assert_eq!(
            parse("-1.5").cmp_exact(&parse("-1.4")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn numeric_literal_positivity_is_exact() {
        assert!(parse("0.0001").is_positive());
        assert!(!parse("0").is_positive());
        assert!(!parse("-0.0001").is_positive());
    }
}
