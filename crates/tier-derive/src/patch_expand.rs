mod enumeration;
mod field;
mod structure;
mod tokens;
mod validate;

use quote::quote;
use syn::{Data, DeriveInput};

use super::serde_attrs::parse_serde_container_attrs;

use self::enumeration::expand_patch_enum;
use self::structure::expand_patch_struct;

pub(crate) fn expand_tier_patch(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let container_attrs = parse_serde_container_attrs(&input.attrs)?;

    let write_tokens = match input.data {
        Data::Struct(data_struct) => expand_patch_struct(data_struct, &container_attrs)?,
        Data::Enum(data_enum) => expand_patch_enum(data_enum, &container_attrs)?,
        Data::Union(union) => {
            return Err(syn::Error::new_spanned(
                union.union_token,
                "TierPatch cannot be derived for unions",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics ::tier::TierPatch for #ident #ty_generics #where_clause {
            fn write_layer(
                &self,
                __tier_builder: &mut ::tier::patch::PatchLayerBuilder,
                __tier_prefix: &str,
            ) -> ::std::result::Result<(), ::tier::ConfigError> {
                #write_tokens
                Ok(())
            }
        }
    })
}
