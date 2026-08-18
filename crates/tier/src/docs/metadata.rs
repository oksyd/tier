use serde_json::Value;

use crate::schema::schema_path_explicit_array_segments;
use crate::{ConfigMetadata, MergeStrategy};

use super::{EnvDocEntry, EnvDocOptions};

pub(super) fn apply_field_metadata(
    entry: &mut EnvDocEntry,
    schema: &Value,
    metadata: &ConfigMetadata,
    options: &EnvDocOptions,
) {
    let explicit_array_segments = schema_path_explicit_array_segments(schema, &entry.path);
    let fields =
        metadata.matching_fields_for_path_with_intent(&entry.path, &explicit_array_segments);
    entry.env = options.env_name(&entry.path);

    for field in fields {
        if let Some(env) = &field.env {
            entry.env = env.clone();
        }
        entry.secret |= field.secret;
        if let Some(doc) = &field.doc {
            entry.description = Some(doc.clone());
        }
        if let Some(example) = &field.example {
            entry.example = Some(example.clone());
        }
        if let Some(deprecated) = &field.deprecated {
            entry.deprecated = Some(deprecated.clone());
        }
        for alias in &field.aliases {
            if !entry.aliases.contains(alias) {
                entry.aliases.push(alias.clone());
            }
        }
        entry.has_default |= field.has_default;
        if field.has_default {
            entry.required = false;
        }
    }

    if let Some(effective) =
        metadata.effective_field_for_path_with_intent(&entry.path, &explicit_array_segments)
    {
        entry.merge = effective.merge;
        entry.validations = effective.validations.clone();
        let validation_export = effective.validation_export();
        entry.validation_levels = validation_export.levels;
        entry.validation_messages = validation_export.messages;
        entry.validation_tags = validation_export.tags;
    }

    if let Some(policy) =
        metadata.effective_source_policy_for_path_with_intent(&entry.path, &explicit_array_segments)
    {
        entry.allowed_sources = policy.allowed_sources_vec();
        entry.denied_sources = policy.denied_sources_vec();
    }

    if entry.secret && entry.example.is_some() {
        entry.example = Some("<secret>".to_owned());
    }
}

pub(super) fn merge_duplicate_env_docs(entries: Vec<EnvDocEntry>) -> Vec<EnvDocEntry> {
    let mut merged = Vec::<EnvDocEntry>::new();

    for entry in entries {
        if let Some(existing) = merged.last_mut()
            && existing.path == entry.path
        {
            merge_env_doc_entry(existing, entry);
        } else {
            merged.push(entry);
        }
    }

    merged
}

fn merge_env_doc_entry(existing: &mut EnvDocEntry, incoming: EnvDocEntry) {
    existing.required |= incoming.required;
    existing.secret |= incoming.secret;
    existing.has_default |= incoming.has_default;

    existing.ty = merge_env_doc_types(&existing.ty, &incoming.ty);
    if existing.description.is_none() {
        existing.description = incoming.description;
    }
    if existing.example.is_none() {
        existing.example = incoming.example;
    }
    if existing.deprecated.is_none() {
        existing.deprecated = incoming.deprecated;
    }
    if existing.aliases.is_empty() {
        existing.aliases = incoming.aliases;
    } else {
        for alias in incoming.aliases {
            if !existing.aliases.contains(&alias) {
                existing.aliases.push(alias);
            }
        }
    }
    if existing.merge == MergeStrategy::Merge && incoming.merge != MergeStrategy::Merge {
        existing.merge = incoming.merge;
    }
    if !incoming.allowed_sources.is_empty() {
        existing.allowed_sources = incoming.allowed_sources;
    }
    if !incoming.denied_sources.is_empty() {
        existing.denied_sources = incoming.denied_sources;
    }
    for rule in incoming.validations {
        if !existing.validations.contains(&rule) {
            existing.validations.push(rule);
        }
    }
    existing
        .validation_levels
        .extend(incoming.validation_levels);
    existing
        .validation_messages
        .extend(incoming.validation_messages);
    existing.validation_tags.extend(incoming.validation_tags);
}

fn merge_env_doc_types(existing: &str, incoming: &str) -> String {
    if existing == incoming {
        return existing.to_owned();
    }

    let mut merged = Vec::<String>::new();
    for ty in [existing, incoming]
        .into_iter()
        .flat_map(|value| value.split(" | "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !merged.iter().any(|existing| existing == ty) {
            merged.push(ty.to_owned());
        }
    }

    if merged.is_empty() {
        "unknown".to_owned()
    } else {
        merged.join(" | ")
    }
}

pub(in crate::docs) fn apply_local_schema_entry_overrides(
    path: &str,
    required: bool,
    object: &serde_json::Map<String, Value>,
    docs: &mut [EnvDocEntry],
) {
    if path.is_empty() {
        return;
    }

    let description = object
        .get("description")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let secret = object
        .get("writeOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || object
            .get("x-tier-secret")
            .and_then(Value::as_bool)
            .unwrap_or(false);

    if !required && !secret && description.is_none() {
        return;
    }

    for entry in docs.iter_mut().filter(|entry| entry.path == path) {
        entry.required |= required;
        entry.secret |= secret;
        if let Some(description) = &description {
            entry.description = Some(description.clone());
        }
    }
}
