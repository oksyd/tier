use quote::{format_ident, quote};
use syn::{Data, Fields, LitStr};

use crate::serde_attrs::{SerdeContainerAttrs, SerdeFieldContext, parse_serde_field_attrs};
use crate::syntax::unraw;

pub(super) fn expand_path_constants(
    data: &Data,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let Data::Struct(data_struct) = data else {
        return Ok(Vec::new());
    };
    let Fields::Named(fields) = &data_struct.fields else {
        return Ok(Vec::new());
    };

    let mut constants = Vec::new();
    let context = SerdeFieldContext::for_struct(container_attrs);
    for field in &fields.named {
        let Some(field_ident) = &field.ident else {
            continue;
        };
        let serde_attrs = parse_serde_field_attrs(&field.attrs, field_ident, context)?;
        if serde_attrs.flatten || serde_attrs.skip_metadata {
            continue;
        }
        let const_ident = format_ident!("PATH_{}", screaming_snake_ident(&unraw(field_ident)));
        let path = LitStr::new(&serde_attrs.canonical_name, field_ident.span());
        constants.push(quote! {
            /// Dot-delimited path constant for this direct config field.
            pub const #const_ident: &'static str = #path;
        });
    }

    Ok(constants)
}

fn screaming_snake_ident(value: &str) -> String {
    let mut rendered = String::new();
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 {
            rendered.push('_');
        }
        rendered.push(ch.to_ascii_uppercase());
    }
    rendered
}
