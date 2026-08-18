use std::collections::BTreeSet;

use super::super::FieldMetadata;

impl FieldMetadata {
    pub(crate) fn set_path_with_array_intent(
        &mut self,
        path: String,
        explicit_array_segments: BTreeSet<usize>,
    ) {
        self.path = path;
        self.path_explicit_array_segments = explicit_array_segments;
    }

    pub(crate) fn try_map_paths<E>(
        &mut self,
        map: impl Fn(&str, &BTreeSet<usize>) -> Result<String, E>,
    ) -> Result<(), E> {
        let path = map(&self.path, &self.path_explicit_array_segments)?;
        let mut mapped_aliases = Vec::with_capacity(self.aliases.len());
        let mut mapped_intents = std::collections::BTreeMap::new();

        for alias in &self.aliases {
            let intent = self
                .alias_explicit_array_segments
                .get(alias)
                .cloned()
                .unwrap_or_default();
            let mapped = map(alias, &intent)?;
            if !intent.is_empty() {
                mapped_intents.insert(mapped.clone(), intent);
            }
            mapped_aliases.push(mapped);
        }

        self.path = path;
        self.aliases = mapped_aliases;
        self.alias_explicit_array_segments = mapped_intents;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FieldMetadata;

    #[test]
    fn mapping_paths_keeps_alias_array_intent_keyed_by_the_mapped_alias() {
        let mut field = FieldMetadata::new("users[00].password").alias("users[00].legacyPassword");

        field
            .try_map_paths(|path, _| Ok::<_, ()>(path.replace(".00.", ".0.")))
            .expect("path mapping succeeds");

        assert_eq!(field.path, "users.0.password");
        assert_eq!(field.aliases, ["users.0.legacyPassword"]);
        assert_eq!(
            field.alias_explicit_array_segments["users.0.legacyPassword"],
            std::collections::BTreeSet::from([1])
        );
    }
}
