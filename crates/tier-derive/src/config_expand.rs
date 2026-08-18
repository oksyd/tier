mod docs;
mod enumeration;
mod field;
mod path_constants;
mod structure;
mod validate;

use quote::quote;
use syn::{Data, DeriveInput};

use super::attr::parse_tier_container_attrs;
use super::container_codegen::container_check_tokens;
use super::serde_attrs::parse_serde_container_attrs;

use self::enumeration::expand_enum_metadata;
use self::path_constants::expand_path_constants;
use self::structure::expand_struct_metadata;

pub(crate) fn expand_tier_config(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let tier_attrs = parse_tier_container_attrs(&input.attrs)?;
    let ident = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let container_attrs = parse_serde_container_attrs(&input.attrs)?;
    let path_constant_tokens = expand_path_constants(&input.data, &container_attrs)?;
    let field_tokens = match input.data {
        Data::Struct(data_struct) => expand_struct_metadata(data_struct, &container_attrs)?,
        Data::Enum(data_enum) => expand_enum_metadata(data_enum, &container_attrs)?,
        Data::Union(union) => {
            return Err(syn::Error::new_spanned(
                union.union_token,
                "TierConfig cannot be derived for unions",
            ));
        }
    };
    let check_tokens = container_check_tokens(&tier_attrs);

    Ok(quote! {
        impl #impl_generics ::tier::TierMetadata for #ident #ty_generics #where_clause {
            fn metadata() -> ::tier::ConfigMetadata {
                let mut metadata = ::tier::ConfigMetadata::new();
                #(#field_tokens)*
                #(#check_tokens)*
                metadata
            }
        }

        impl #impl_generics #ident #ty_generics #where_clause {
            #(#path_constant_tokens)*
        }
    })
}
