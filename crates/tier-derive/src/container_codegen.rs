use quote::quote;
use syn::LitStr;

use super::attr::{
    ContainerPathListSpec, ContainerPathSpec, ContainerValidationCheck, TierContainerAttrs,
};

pub(crate) fn container_check_tokens(attrs: &TierContainerAttrs) -> Vec<proc_macro2::TokenStream> {
    attrs
        .checks
        .iter()
        .map(|check| match check {
            ContainerValidationCheck::AtLeastOneOf(paths) => {
                let paths = container_paths_tokens(paths);
                quote! {
                    metadata.push_check(::tier::ValidationCheck::AtLeastOneOf {
                        paths: #paths,
                    });
                }
            }
            ContainerValidationCheck::ExactlyOneOf(paths) => {
                let paths = container_paths_tokens(paths);
                quote! {
                    metadata.push_check(::tier::ValidationCheck::ExactlyOneOf {
                        paths: #paths,
                    });
                }
            }
            ContainerValidationCheck::MutuallyExclusive(paths) => {
                let paths = container_paths_tokens(paths);
                quote! {
                    metadata.push_check(::tier::ValidationCheck::MutuallyExclusive {
                        paths: #paths,
                    });
                }
            }
            ContainerValidationCheck::RequiredWith { path, requires } => {
                let path = container_path_tokens(path);
                let requires = container_paths_tokens(requires);
                quote! {
                    metadata.push_check(::tier::ValidationCheck::RequiredWith {
                        path: #path,
                        requires: #requires,
                    });
                }
            }
            ContainerValidationCheck::RequiredIf {
                path,
                equals,
                requires,
            } => {
                let path = container_path_tokens(path);
                let requires = container_paths_tokens(requires);
                quote! {
                    metadata.push_check(::tier::ValidationCheck::RequiredIf {
                        path: #path,
                        equals: ::tier::ValidationValue::from(#equals),
                        requires: #requires,
                    });
                }
            }
        })
        .collect()
}

fn container_path_tokens(path: &ContainerPathSpec) -> proc_macro2::TokenStream {
    match path {
        ContainerPathSpec::String(path) => {
            let path = LitStr::new(path, proc_macro2::Span::call_site());
            quote! { ::std::string::String::from(#path) }
        }
        ContainerPathSpec::Expr(path) => quote! { ::std::string::String::from(#path) },
    }
}

fn container_paths_tokens(paths: &ContainerPathListSpec) -> proc_macro2::TokenStream {
    match paths {
        ContainerPathListSpec::Strings(paths) => {
            let paths = paths
                .iter()
                .map(|path| LitStr::new(path, proc_macro2::Span::call_site()))
                .collect::<Vec<_>>();
            quote! { ::std::vec![#(::std::string::String::from(#paths)),*] }
        }
        ContainerPathListSpec::Exprs(paths) => {
            quote! { ::std::vec![#(::std::string::String::from(#paths)),*] }
        }
    }
}
