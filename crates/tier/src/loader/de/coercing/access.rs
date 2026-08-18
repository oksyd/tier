use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use serde::de::{
    DeserializeSeed, Error as _, IntoDeserializer, MapAccess, SeqAccess,
    value::Error as ValueDeError,
};
use serde_json::Value;

use crate::path::join_path;

use super::CoercingDeserializer;

pub(super) struct CoercingSeqAccess<'a, I> {
    iter: I,
    parent_path: String,
    string_coercion_paths: &'a BTreeSet<String>,
    known_paths: Option<&'a RefCell<BTreeSet<String>>>,
    ignored_paths: Option<&'a RefCell<Vec<String>>>,
    coerced_values: Option<&'a RefCell<BTreeMap<String, Value>>>,
}

impl<'a, I> CoercingSeqAccess<'a, I> {
    pub(super) fn new(
        iter: I,
        parent_path: String,
        string_coercion_paths: &'a BTreeSet<String>,
        known_paths: Option<&'a RefCell<BTreeSet<String>>>,
        ignored_paths: Option<&'a RefCell<Vec<String>>>,
        coerced_values: Option<&'a RefCell<BTreeMap<String, Value>>>,
    ) -> Self {
        Self {
            iter,
            parent_path,
            string_coercion_paths,
            known_paths,
            ignored_paths,
            coerced_values,
        }
    }
}

impl<'de, 'a, I> SeqAccess<'de> for CoercingSeqAccess<'a, I>
where
    'a: 'de,
    I: Iterator<Item = (usize, &'a Value)>,
{
    type Error = ValueDeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some((index, value)) = self.iter.next() else {
            return Ok(None);
        };
        let path = join_path(&self.parent_path, &index.to_string());
        seed.deserialize(CoercingDeserializer::new(
            value,
            path,
            self.string_coercion_paths,
            self.known_paths,
            self.ignored_paths,
            self.coerced_values,
        ))
        .map(Some)
    }
}

pub(super) struct CoercingMapAccess<'a, I> {
    iter: I,
    current: Option<(&'a str, &'a Value)>,
    parent_path: String,
    string_coercion_paths: &'a BTreeSet<String>,
    known_paths: Option<&'a RefCell<BTreeSet<String>>>,
    ignored_paths: Option<&'a RefCell<Vec<String>>>,
    coerced_values: Option<&'a RefCell<BTreeMap<String, Value>>>,
}

impl<'a, I> CoercingMapAccess<'a, I> {
    pub(super) fn new(
        iter: I,
        parent_path: String,
        string_coercion_paths: &'a BTreeSet<String>,
        known_paths: Option<&'a RefCell<BTreeSet<String>>>,
        ignored_paths: Option<&'a RefCell<Vec<String>>>,
        coerced_values: Option<&'a RefCell<BTreeMap<String, Value>>>,
    ) -> Self {
        Self {
            iter,
            current: None,
            parent_path,
            string_coercion_paths,
            known_paths,
            ignored_paths,
            coerced_values,
        }
    }
}

impl<'de, 'a, I> MapAccess<'de> for CoercingMapAccess<'a, I>
where
    'a: 'de,
    I: Iterator<Item = (&'a String, &'a Value)>,
{
    type Error = ValueDeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.iter.next() else {
            return Ok(None);
        };
        self.current = Some((key.as_str(), value));
        seed.deserialize(key.as_str().into_deserializer()).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.current.take() else {
            return Err(ValueDeError::custom(
                "map value requested before key was deserialized",
            ));
        };
        let path = join_path(&self.parent_path, key);
        seed.deserialize(CoercingDeserializer::new(
            value,
            path,
            self.string_coercion_paths,
            self.known_paths,
            self.ignored_paths,
            self.coerced_values,
        ))
    }
}
