use syn::{Field, FieldsUnnamed, spanned::Spanned};

pub(crate) fn named_field_ident(field: &Field) -> syn::Result<syn::Ident> {
    field.ident.clone().ok_or_else(|| {
        syn::Error::new_spanned(field, "expected a named field while expanding tier derive")
    })
}

pub(crate) fn single_unnamed_field(
    fields: FieldsUnnamed,
    message: &'static str,
) -> syn::Result<Field> {
    if fields.unnamed.len() != 1 {
        return Err(syn::Error::new_spanned(fields, message));
    }

    let span = fields.span();
    fields
        .unnamed
        .into_iter()
        .next()
        .ok_or_else(|| syn::Error::new(span, message))
}
