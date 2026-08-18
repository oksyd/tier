#![warn(missing_docs)]
#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attr;
mod config_expand;
mod container_codegen;
mod field;
mod field_codegen;
mod patch_expand;
mod serde_attrs;
mod syntax;
mod ty;

use self::config_expand::expand_tier_config;
use self::patch_expand::expand_tier_patch;

#[proc_macro_derive(TierConfig, attributes(tier, serde))]
/// Derives `tier::TierMetadata` for nested configuration structs.
pub fn derive_tier_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_tier_config(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(TierPatch, attributes(tier, serde))]
/// Derives `tier::TierPatch` for typed sparse override structs.
pub fn derive_tier_patch(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_tier_patch(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
