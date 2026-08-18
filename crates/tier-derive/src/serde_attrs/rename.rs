#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl RenameRule {
    pub(super) fn parse(value: &str, span: proc_macro2::Span) -> syn::Result<Self> {
        match value {
            "lowercase" => Ok(Self::Lower),
            "UPPERCASE" => Ok(Self::Upper),
            "PascalCase" => Ok(Self::Pascal),
            "camelCase" => Ok(Self::Camel),
            "snake_case" => Ok(Self::Snake),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnake),
            "kebab-case" => Ok(Self::Kebab),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebab),
            _ => Err(syn::Error::new(
                span,
                "unsupported serde rename rule for TierConfig",
            )),
        }
    }

    pub(super) fn apply_to_field(self, value: &str) -> String {
        match self {
            Self::Lower | Self::Snake => value.to_owned(),
            Self::Upper | Self::ScreamingSnake => value.to_ascii_uppercase(),
            Self::Pascal => {
                let mut output = String::new();
                let mut capitalize = true;
                for ch in value.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        output.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        output.push(ch);
                    }
                }
                output
            }
            Self::Camel => {
                let pascal = Self::Pascal.apply_to_field(value);
                lowercase_first_char(&pascal)
            }
            Self::Kebab => value.replace('_', "-"),
            Self::ScreamingKebab => value.replace('_', "-").to_ascii_uppercase(),
        }
    }

    pub(super) fn apply_to_variant(self, value: &str) -> String {
        match self {
            Self::Lower => value.to_ascii_lowercase(),
            Self::Upper => value.to_ascii_uppercase(),
            Self::Pascal => value.to_owned(),
            Self::Camel => lowercase_first_char(value),
            Self::Snake => {
                let mut output = String::new();
                for (index, ch) in value.char_indices() {
                    if index > 0 && ch.is_uppercase() {
                        output.push('_');
                    }
                    output.push(ch.to_ascii_lowercase());
                }
                output
            }
            Self::ScreamingSnake => Self::Snake.apply_to_variant(value).to_ascii_uppercase(),
            Self::Kebab => Self::Snake.apply_to_variant(value).replace('_', "-"),
            Self::ScreamingKebab => Self::Kebab.apply_to_variant(value).to_ascii_uppercase(),
        }
    }
}

fn lowercase_first_char(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    let mut output = first.to_ascii_lowercase().to_string();
    output.push_str(chars.as_str());
    output
}
