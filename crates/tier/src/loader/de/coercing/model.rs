use std::cell::RefCell;
use std::collections::BTreeSet;

use serde::de::{Error as _, Unexpected, Visitor, value::Error as ValueDeError};
use serde_json::Value;

use crate::path::normalize_path;

use super::unexpected::unexpected_value;

pub(in crate::loader) struct CoercingDeserializer<'a> {
    pub(super) value: &'a Value,
    pub(super) path: String,
    pub(super) string_coercion_paths: &'a BTreeSet<String>,
    pub(super) known_paths: Option<&'a RefCell<BTreeSet<String>>>,
    pub(super) ignored_paths: Option<&'a RefCell<Vec<String>>>,
}

impl<'a> CoercingDeserializer<'a> {
    pub(in crate::loader) fn new(
        value: &'a Value,
        path: impl Into<String>,
        string_coercion_paths: &'a BTreeSet<String>,
        known_paths: Option<&'a RefCell<BTreeSet<String>>>,
        ignored_paths: Option<&'a RefCell<Vec<String>>>,
    ) -> Self {
        Self {
            value,
            path: path.into(),
            string_coercion_paths,
            known_paths,
            ignored_paths,
        }
    }

    pub(super) fn coercible_string(&self) -> Option<&'a str> {
        match self.value {
            Value::String(value) if self.string_coercion_paths.contains(&self.path) => Some(value),
            _ => None,
        }
    }

    pub(super) fn invalid_type<'de, V>(&self, visitor: &V) -> ValueDeError
    where
        V: Visitor<'de>,
    {
        ValueDeError::invalid_type(unexpected_value(self.value), visitor)
    }

    pub(super) fn invalid_string_type<'de, V>(&self, raw: &str, visitor: &V) -> ValueDeError
    where
        V: Visitor<'de>,
    {
        ValueDeError::invalid_type(Unexpected::Str(raw), visitor)
    }

    pub(super) fn record_known_path(&self, path: &str) {
        if let Some(known_paths) = self.known_paths {
            let normalized = normalize_path(path);
            if !normalized.is_empty() {
                known_paths.borrow_mut().insert(normalized);
            }
        }
    }

    pub(super) fn record_ignored_path(&self, path: &str) {
        if let Some(ignored_paths) = self.ignored_paths {
            let normalized = normalize_path(path);
            if !normalized.is_empty() {
                ignored_paths.borrow_mut().push(normalized);
            }
        }
    }
}
