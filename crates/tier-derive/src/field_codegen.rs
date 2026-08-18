use quote::quote;
use syn::LitStr;

use super::{attr::TierAttrs, serde_attrs::SerdeFieldAttrs};

pub(crate) fn direct_field_metadata_tokens(
    accumulator: &proc_macro2::Ident,
    field_name: &LitStr,
    aliases: &[LitStr],
    serde_attrs: &SerdeFieldAttrs,
    attrs: &TierAttrs,
    secret_type: bool,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut builder = quote! {
        ::tier::FieldMetadata::new(#field_name)
    };

    for alias in aliases {
        builder = quote! { #builder.alias(#alias) };
    }
    if attrs.secret || secret_type {
        builder = quote! { #builder.secret() };
    }
    if let Some(env) = &attrs.env {
        let env = LitStr::new(env, field_name.span());
        builder = quote! { #builder.env(#env) };
    }
    if let Some(doc) = &attrs.doc {
        let doc = LitStr::new(doc, field_name.span());
        builder = quote! { #builder.doc(#doc) };
    }
    if let Some(example) = &attrs.example {
        let example = LitStr::new(example, field_name.span());
        builder = quote! { #builder.example(#example) };
    }
    if let Some(deprecated) = &attrs.deprecated {
        let deprecated = LitStr::new(deprecated, field_name.span());
        builder = quote! { #builder.deprecated(#deprecated) };
    }
    if serde_attrs.has_default {
        builder = quote! { #builder.defaulted() };
    }
    if let Some(merge) = &attrs.merge {
        let merge_strategy = match merge.as_str() {
            "merge" => quote! { ::tier::MergeStrategy::Merge },
            "replace" => quote! { ::tier::MergeStrategy::Replace },
            "append" => quote! { ::tier::MergeStrategy::Append },
            _ => {
                return Err(syn::Error::new(
                    field_name.span(),
                    "unsupported tier merge strategy, expected merge|replace|append",
                ));
            }
        };
        builder = quote! { #builder.merge_strategy(#merge_strategy) };
    }
    if !attrs.sources.is_empty() {
        let sources = attrs
            .sources
            .iter()
            .map(|source| source.tokens())
            .collect::<Vec<_>>();
        builder = quote! { #builder.allow_sources([#(#sources),*]) };
    }
    if !attrs.deny_sources.is_empty() {
        let sources = attrs
            .deny_sources
            .iter()
            .map(|source| source.tokens())
            .collect::<Vec<_>>();
        builder = quote! { #builder.deny_sources([#(#sources),*]) };
    }
    if attrs.non_empty {
        builder = quote! { #builder.non_empty() };
    }
    if let Some(min) = &attrs.min {
        let min = &min.tokens;
        builder = quote! { #builder.min(#min) };
    }
    if let Some(max) = &attrs.max {
        let max = &max.tokens;
        builder = quote! { #builder.max(#max) };
    }
    if let Some(min_length) = attrs.min_length {
        builder = quote! { #builder.min_length(#min_length) };
    }
    if let Some(max_length) = attrs.max_length {
        builder = quote! { #builder.max_length(#max_length) };
    }
    if let Some(min_items) = attrs.min_items {
        builder = quote! { #builder.min_items(#min_items) };
    }
    if let Some(max_items) = attrs.max_items {
        builder = quote! { #builder.max_items(#max_items) };
    }
    if let Some(min_properties) = attrs.min_properties {
        builder = quote! { #builder.min_properties(#min_properties) };
    }
    if let Some(max_properties) = attrs.max_properties {
        builder = quote! { #builder.max_properties(#max_properties) };
    }
    if let Some(multiple_of) = &attrs.multiple_of {
        let multiple_of = &multiple_of.tokens;
        builder = quote! { #builder.multiple_of(#multiple_of) };
    }
    if let Some(pattern) = &attrs.pattern {
        let pattern = LitStr::new(pattern, field_name.span());
        builder = quote! { #builder.pattern(#pattern) };
    }
    if attrs.unique_items {
        builder = quote! { #builder.unique_items() };
    }
    if !attrs.one_of.is_empty() {
        let one_of = &attrs.one_of;
        builder = quote! { #builder.one_of([#(#one_of),*]) };
    }
    if attrs.hostname {
        builder = quote! { #builder.hostname() };
    }
    if attrs.url {
        builder = quote! { #builder.url() };
    }
    if attrs.email {
        builder = quote! { #builder.email() };
    }
    if attrs.ip_addr {
        builder = quote! { #builder.ip_addr() };
    }
    if attrs.socket_addr {
        builder = quote! { #builder.socket_addr() };
    }
    if attrs.absolute_path {
        builder = quote! { #builder.absolute_path() };
    }
    if let Some(env_decode) = &attrs.env_decode {
        let env_decode = match env_decode.as_str() {
            "csv" => quote! { ::tier::EnvDecoder::Csv },
            "path_list" => quote! { ::tier::EnvDecoder::PathList },
            "key_value_map" => quote! { ::tier::EnvDecoder::KeyValueMap },
            "whitespace" => quote! { ::tier::EnvDecoder::Whitespace },
            _ => {
                return Err(syn::Error::new(
                    field_name.span(),
                    "unsupported tier env decoder, expected csv|path_list|key_value_map|whitespace",
                ));
            }
        };
        builder = quote! { #builder.env_decoder(#env_decode) };
    }
    for (rule, message) in &attrs.validation_messages {
        let rule = LitStr::new(rule, field_name.span());
        let message = LitStr::new(message, field_name.span());
        builder = quote! { #builder.validation_message(#rule, #message) };
    }
    for (rule, level) in &attrs.validation_levels {
        let rule = LitStr::new(rule, field_name.span());
        let level = match level.as_str() {
            "error" => quote! { ::tier::ValidationLevel::Error },
            "warn" | "warning" => quote! { ::tier::ValidationLevel::Warning },
            _ => {
                return Err(syn::Error::new(
                    field_name.span(),
                    "unsupported validation level, expected error|warning",
                ));
            }
        };
        builder = quote! { #builder.validation_level(#rule, #level) };
    }
    for (rule, tags) in &attrs.validation_tags {
        let rule = LitStr::new(rule, field_name.span());
        let tags = tags
            .iter()
            .map(|tag| LitStr::new(tag, field_name.span()))
            .collect::<Vec<_>>();
        builder = quote! { #builder.validation_tags(#rule, [#(#tags),*]) };
    }

    Ok(quote! {
        #accumulator.push(#builder);
    })
}
