use std::cmp::Ordering;

use quote::quote;
use syn::Expr;

#[derive(Debug, Default)]
pub(crate) struct TierAttrs {
    pub(crate) secret: bool,
    pub(crate) leaf: bool,
    pub(crate) sources: Vec<TierSourceKind>,
    pub(crate) deny_sources: Vec<TierSourceKind>,
    pub(crate) env: Option<String>,
    pub(crate) doc: Option<String>,
    pub(crate) example: Option<String>,
    pub(crate) deprecated: Option<String>,
    pub(crate) merge: Option<String>,
    pub(crate) non_empty: bool,
    pub(crate) min: Option<NumericLiteral>,
    pub(crate) max: Option<NumericLiteral>,
    pub(crate) min_length: Option<usize>,
    pub(crate) max_length: Option<usize>,
    pub(crate) min_items: Option<usize>,
    pub(crate) max_items: Option<usize>,
    pub(crate) min_properties: Option<usize>,
    pub(crate) max_properties: Option<usize>,
    pub(crate) multiple_of: Option<NumericLiteral>,
    pub(crate) pattern: Option<String>,
    pub(crate) unique_items: bool,
    pub(crate) one_of: Vec<Expr>,
    pub(crate) hostname: bool,
    pub(crate) url: bool,
    pub(crate) email: bool,
    pub(crate) ip_addr: bool,
    pub(crate) socket_addr: bool,
    pub(crate) absolute_path: bool,
    pub(crate) env_decode: Option<String>,
    pub(crate) validation_messages: Vec<(String, String)>,
    pub(crate) validation_levels: Vec<(String, String)>,
    pub(crate) validation_tags: Vec<(String, Vec<String>)>,
}

impl TierAttrs {
    pub(crate) fn has_any(&self) -> bool {
        self.secret
            || self.leaf
            || !self.sources.is_empty()
            || !self.deny_sources.is_empty()
            || self.env.is_some()
            || self.doc.is_some()
            || self.example.is_some()
            || self.deprecated.is_some()
            || self.merge.is_some()
            || self.non_empty
            || self.min.is_some()
            || self.max.is_some()
            || self.min_length.is_some()
            || self.max_length.is_some()
            || self.min_items.is_some()
            || self.max_items.is_some()
            || self.min_properties.is_some()
            || self.max_properties.is_some()
            || self.multiple_of.is_some()
            || self.pattern.is_some()
            || self.unique_items
            || !self.one_of.is_empty()
            || self.hostname
            || self.url
            || self.email
            || self.ip_addr
            || self.socket_addr
            || self.absolute_path
            || self.env_decode.is_some()
            || !self.validation_messages.is_empty()
            || !self.validation_levels.is_empty()
            || !self.validation_tags.is_empty()
    }
}

#[derive(Debug, Default)]
pub(crate) struct PatchAttrs {
    pub(crate) path: Option<String>,
    pub(crate) path_expr: Option<Expr>,
    pub(crate) nested: bool,
    pub(crate) skip: bool,
}

impl PatchAttrs {
    pub(crate) fn has_any(&self) -> bool {
        self.path.is_some() || self.path_expr.is_some() || self.nested || self.skip
    }

    pub(crate) fn has_non_skip(&self) -> bool {
        self.path.is_some() || self.path_expr.is_some() || self.nested
    }
}

#[derive(Debug, Default)]
pub(crate) struct TierContainerAttrs {
    pub(crate) checks: Vec<ContainerValidationCheck>,
}

#[derive(Debug, Clone)]
pub(crate) struct NumericLiteral {
    pub(crate) tokens: proc_macro2::TokenStream,
    pub(crate) value: NumericLiteralValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumericLiteralValue {
    negative: bool,
    numerator: u128,
    denominator: u128,
}

impl NumericLiteralValue {
    pub(crate) fn new(negative: bool, numerator: u128, denominator: u128) -> Self {
        debug_assert!(denominator > 0);
        let divisor = gcd_u128(numerator, denominator);
        let numerator = numerator / divisor;
        Self {
            negative: negative && numerator != 0,
            numerator,
            denominator: denominator / divisor,
        }
    }

    pub(crate) fn is_positive(&self) -> bool {
        !self.negative && self.numerator > 0
    }

    pub(crate) fn cmp_exact(&self, other: &Self) -> Option<Ordering> {
        match (self.negative, other.negative) {
            (true, false) => return Some(Ordering::Less),
            (false, true) => return Some(Ordering::Greater),
            _ => {}
        }

        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        let ordering = left.cmp(&right);
        Some(if self.negative {
            ordering.reverse()
        } else {
            ordering
        })
    }
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TierSourceKind {
    Default,
    File,
    Environment,
    Arguments,
    Normalization,
    Custom,
}

impl TierSourceKind {
    pub(super) fn parse(value: &str, span: proc_macro2::Span) -> syn::Result<Self> {
        match value {
            "default" => Ok(Self::Default),
            "file" => Ok(Self::File),
            "env" | "environment" => Ok(Self::Environment),
            "cli" | "arguments" => Ok(Self::Arguments),
            "normalize" | "normalization" => Ok(Self::Normalization),
            "custom" => Ok(Self::Custom),
            _ => Err(syn::Error::new(
                span,
                "unsupported tier source kind, expected default|file|env|cli|normalize|custom",
            )),
        }
    }

    pub(crate) fn tokens(self) -> proc_macro2::TokenStream {
        match self {
            Self::Default => quote! { ::tier::SourceKind::Default },
            Self::File => quote! { ::tier::SourceKind::File },
            Self::Environment => quote! { ::tier::SourceKind::Environment },
            Self::Arguments => quote! { ::tier::SourceKind::Arguments },
            Self::Normalization => quote! { ::tier::SourceKind::Normalization },
            Self::Custom => quote! { ::tier::SourceKind::Custom },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ContainerPathSpec {
    String(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub(crate) enum ContainerPathListSpec {
    Strings(Vec<String>),
    Exprs(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub(crate) enum ContainerValidationCheck {
    AtLeastOneOf(ContainerPathListSpec),
    ExactlyOneOf(ContainerPathListSpec),
    MutuallyExclusive(ContainerPathListSpec),
    RequiredWith {
        path: ContainerPathSpec,
        requires: ContainerPathListSpec,
    },
    RequiredIf {
        path: ContainerPathSpec,
        equals: Expr,
        requires: ContainerPathListSpec,
    },
}
