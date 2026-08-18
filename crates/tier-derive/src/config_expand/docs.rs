use syn::{Attribute, Expr, Lit, Meta};

pub(super) fn doc_comment(attributes: &[Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attribute in attributes {
        if !attribute.path().is_ident("doc") {
            continue;
        }
        let Meta::NameValue(name_value) = &attribute.meta else {
            continue;
        };
        let Expr::Lit(expr_lit) = &name_value.value else {
            continue;
        };
        let Lit::Str(literal) = &expr_lit.lit else {
            continue;
        };
        let line = literal.value().trim().to_owned();
        if !line.is_empty() {
            lines.push(line);
        }
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}
