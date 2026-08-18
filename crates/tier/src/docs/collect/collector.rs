use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::docs::EnvDocEntry;
use crate::schema::{inlined_schema_ref, merged_object_level_property_names};

use super::array as array_docs;
use super::combinator::{COMBINATOR_KEYWORDS, branch_is_null, branch_required};
use super::entry::{any_leaf_doc_entry, schema_leaf_doc_entry, unknown_leaf_doc_entry};
use super::object as object_docs;
use super::path::child_path;
use crate::docs::metadata::apply_local_schema_entry_overrides;

pub(super) fn collect_env_docs(schema: &Value, docs: &mut Vec<EnvDocEntry>) {
    EnvDocCollector::new(schema, docs).collect_root();
}

struct EnvDocCollector<'a, 'docs> {
    root: &'a Value,
    docs: &'docs mut Vec<EnvDocEntry>,
    visited_refs: BTreeSet<String>,
}

impl<'a, 'docs> EnvDocCollector<'a, 'docs> {
    fn new(root: &'a Value, docs: &'docs mut Vec<EnvDocEntry>) -> Self {
        Self {
            root,
            docs,
            visited_refs: BTreeSet::new(),
        }
    }

    fn collect_root(&mut self) {
        self.collect(self.root, "", true, None);
    }

    fn collect(
        &mut self,
        schema: &Value,
        path: &str,
        required: bool,
        scope_reserved_keys: Option<&BTreeSet<String>>,
    ) {
        if self.collect_ref(schema, path, required, scope_reserved_keys) {
            return;
        }

        let Some(object) = schema.as_object() else {
            self.collect_non_object_leaf(schema, path, required);
            return;
        };

        self.collect_object(schema, object, path, required, scope_reserved_keys);
    }

    fn collect_ref(
        &mut self,
        schema: &Value,
        path: &str,
        required: bool,
        scope_reserved_keys: Option<&BTreeSet<String>>,
    ) -> bool {
        let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
            return false;
        };

        if self.visited_refs.insert(reference.to_owned()) {
            if let Some(inlined) = inlined_schema_ref(schema, self.root) {
                self.collect(&inlined, path, required, scope_reserved_keys);
            }
            self.visited_refs.remove(reference);
        }

        true
    }

    fn collect_non_object_leaf(&mut self, schema: &Value, path: &str, required: bool) {
        match schema {
            Value::Bool(true) if !path.is_empty() => {
                self.docs.push(any_leaf_doc_entry(path, required));
            }
            Value::Bool(false) => {}
            _ if !path.is_empty() => {
                self.docs.push(unknown_leaf_doc_entry(path, required));
            }
            _ => {}
        }
    }

    fn collect_object(
        &mut self,
        schema: &Value,
        object: &Map<String, Value>,
        path: &str,
        required: bool,
        scope_reserved_keys: Option<&BTreeSet<String>>,
    ) {
        let reserved_keys =
            merged_object_level_property_names(schema, self.root, scope_reserved_keys);
        let required_properties = object_docs::required_properties(object);
        let min_properties = object_docs::min_properties(object);
        let traversed_combinator = self.collect_combinators(object, path, required, &reserved_keys);
        let mut traversed_children = traversed_combinator;

        let properties = object.get("properties").and_then(Value::as_object);
        if let Some(properties) = properties {
            traversed_children = true;
            self.collect_properties(
                object,
                properties,
                path,
                required,
                min_properties,
                &required_properties,
            );
        }

        let known_property_count = properties.map_or(0, Map::len);
        let max_properties = object_docs::max_properties(object);
        let required_dynamic_properties = object_docs::required_dynamic_properties(
            required,
            min_properties,
            known_property_count,
        );
        let allows_dynamic_properties =
            object_docs::allows_dynamic_properties(max_properties, required_properties.len());

        traversed_children |= self.collect_dynamic_object_children(
            object,
            path,
            required_dynamic_properties,
            allows_dynamic_properties,
            &reserved_keys,
        );
        traversed_children |= self.collect_array_children(object, path, required);

        if traversed_children {
            if traversed_combinator {
                apply_local_schema_entry_overrides(path, required, object, self.docs);
            }
            return;
        }

        if !path.is_empty() {
            self.docs
                .push(schema_leaf_doc_entry(path, required, object));
        }
    }

    fn collect_combinators(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        required: bool,
        reserved_keys: &BTreeSet<String>,
    ) -> bool {
        let mut traversed = false;
        for keyword in COMBINATOR_KEYWORDS {
            if let Some(children) = object.get(keyword).and_then(Value::as_array) {
                traversed = true;
                for child in children {
                    if !branch_is_null(child) {
                        self.collect(
                            child,
                            path,
                            branch_required(keyword, required),
                            Some(reserved_keys),
                        );
                    }
                }
            }
        }
        traversed
    }

    fn collect_properties(
        &mut self,
        object: &Map<String, Value>,
        properties: &Map<String, Value>,
        path: &str,
        required: bool,
        min_properties: usize,
        required_properties: &BTreeSet<String>,
    ) {
        let max_properties = object_docs::max_properties(object);
        let all_known_properties_required =
            object_docs::all_known_properties_required(object, properties, min_properties);
        let allows_optional_properties =
            object_docs::allows_optional_properties(max_properties, required_properties.len());

        for (key, child_schema) in properties {
            let property_required =
                required && (required_properties.contains(key) || all_known_properties_required);
            if !property_required && !allows_optional_properties {
                continue;
            }

            let next = child_path(path, key);
            self.collect(child_schema, &next, property_required, None);
        }
    }

    fn collect_dynamic_object_children(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        required_dynamic_properties: bool,
        allows_dynamic_properties: bool,
        reserved_keys: &BTreeSet<String>,
    ) -> bool {
        let mut traversed = false;
        if let Some(pattern_properties) = object.get("patternProperties").and_then(Value::as_object)
        {
            traversed = true;
            if let (true, Some(segment)) = (
                allows_dynamic_properties,
                object_docs::dynamic_property_segment(object, self.root, reserved_keys),
            ) {
                let next = child_path(path, &segment);
                for child_schema in pattern_properties.values() {
                    self.collect(child_schema, &next, required_dynamic_properties, None);
                }
            }
        }

        let implicit_additional = Value::Bool(true);
        if let Some(additional) = object_docs::additional_properties_schema(
            object,
            required_dynamic_properties,
            &implicit_additional,
        ) {
            traversed = true;
            if let (true, Some(segment)) = (
                allows_dynamic_properties,
                object_docs::dynamic_property_segment(object, self.root, reserved_keys),
            ) {
                let next = child_path(path, &segment);
                self.collect(additional, &next, required_dynamic_properties, None);
            }
        }

        traversed
    }

    fn collect_array_children(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        required: bool,
    ) -> bool {
        let mut traversed = false;

        traversed |= self.collect_tuple_items(object, path, required, "prefixItems");
        traversed |= self.collect_tuple_items(object, path, required, "items");

        if let Some(items) = array_docs::homogeneous_items_schema(object)
            && array_docs::allows_wildcard_entries(object)
        {
            traversed = true;
            let next = child_path(path, "*");
            self.collect(
                items,
                &next,
                array_docs::additional_items_required(required, object),
                None,
            );
        }

        if let Some(additional) = array_docs::legacy_additional_items_schema(object)
            && array_docs::allows_wildcard_entries(object)
        {
            traversed = true;
            let next = child_path(path, "*");
            self.collect(
                additional,
                &next,
                array_docs::additional_items_required(required, object),
                None,
            );
        }

        if let Some(contains) = array_docs::contains_schema(object)
            && array_docs::allows_wildcard_entries(object)
        {
            traversed = true;
            let next = child_path(path, "*");
            self.collect(
                contains,
                &next,
                array_docs::contains_items_required(required, object, self.root),
                None,
            );
        }

        traversed
    }

    fn collect_tuple_items(
        &mut self,
        object: &Map<String, Value>,
        path: &str,
        required: bool,
        keyword: &str,
    ) -> bool {
        let Some(items) = object.get(keyword).and_then(Value::as_array) else {
            return false;
        };

        for (index, child) in items.iter().enumerate() {
            let next = child_path(path, &index.to_string());
            self.collect(
                child,
                &next,
                array_docs::fixed_item_required(required, object, index),
                None,
            );
        }

        true
    }
}
