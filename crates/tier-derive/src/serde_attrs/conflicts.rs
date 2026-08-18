use std::collections::{HashMap, HashSet};

use syn::{DataEnum, Fields};

use super::model::{
    EnumRepresentation, NonExternalFieldConflicts, SerdeContainerAttrs, SerdeFieldContext,
};
use super::parse::{enum_representation, parse_serde_field_attrs, parse_serde_variant_attrs};
use crate::attr::parse_tier_attrs;

pub(crate) fn non_external_variant_field_conflicts(
    data_enum: &DataEnum,
    container_attrs: &SerdeContainerAttrs,
) -> syn::Result<NonExternalFieldConflicts> {
    let representation = enum_representation(container_attrs)?;
    if matches!(representation, EnumRepresentation::External) {
        return Ok(NonExternalFieldConflicts::default());
    }

    let context = SerdeFieldContext::for_enum_variant_fields(container_attrs);
    let mut counts = HashMap::<String, usize>::new();
    let mut canonical_names = HashSet::new();
    let mut alias_owners = HashMap::<String, HashSet<String>>::new();
    let mut env_owners = HashMap::<String, HashSet<String>>::new();

    for variant in &data_enum.variants {
        let variant_attrs =
            parse_serde_variant_attrs(&variant.attrs, &variant.ident, container_attrs)?;
        if variant_attrs.skip_metadata {
            continue;
        }

        let Fields::Named(fields) = &variant.fields else {
            continue;
        };

        let mut seen = HashSet::new();
        for field in &fields.named {
            let Some(field_ident) = &field.ident else {
                continue;
            };
            let serde_attrs = parse_serde_field_attrs(&field.attrs, field_ident, context)?;
            if serde_attrs.skip_metadata || serde_attrs.flatten {
                continue;
            }
            let tier_attrs = parse_tier_attrs(&field.attrs)?;
            let canonical_name = serde_attrs.canonical_name.clone();
            if seen.insert(canonical_name.clone()) {
                canonical_names.insert(canonical_name.clone());
                *counts.entry(canonical_name.clone()).or_default() += 1;
            }
            for alias in serde_attrs.aliases {
                alias_owners
                    .entry(alias)
                    .or_default()
                    .insert(canonical_name.clone());
            }
            if let Some(env) = tier_attrs.env {
                env_owners
                    .entry(env)
                    .or_default()
                    .insert(canonical_name.clone());
            }
        }
    }

    let skipped_fields = counts
        .into_iter()
        .filter_map(|(path, count)| (count > 1).then_some(path))
        .collect::<HashSet<_>>();

    let skipped_aliases = alias_owners
        .into_iter()
        .filter_map(|(alias, owners)| {
            (owners.len() > 1 || canonical_names.contains(&alias)).then_some(alias)
        })
        .collect::<HashSet<_>>();

    let skipped_envs = env_owners
        .into_iter()
        .filter_map(|(env, owners)| (owners.len() > 1).then_some(env))
        .collect::<HashSet<_>>();

    Ok(NonExternalFieldConflicts {
        skipped_fields,
        skipped_aliases,
        skipped_envs,
    })
}
