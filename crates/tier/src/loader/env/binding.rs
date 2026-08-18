use crate::EnvDecoder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnvBinding {
    pub(super) path: String,
    pub(super) decoder: Option<EnvDecoder>,
    pub(super) fallback: bool,
}

#[derive(Debug, Clone)]
pub(super) struct EnvBindingConflict {
    pub(super) name: String,
    pub(super) first: EnvBinding,
    pub(super) second: EnvBinding,
}
