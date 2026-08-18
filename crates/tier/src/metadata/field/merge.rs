use super::super::FieldMetadata;

impl FieldMetadata {
    pub(in crate::metadata) fn merge_from(&mut self, other: Self) {
        self.aliases.extend(other.aliases);
        self.aliases.sort();
        self.aliases.dedup();
        self.alias_explicit_array_segments
            .extend(other.alias_explicit_array_segments);
        self.secret |= other.secret;
        if let Some(env) = other.env {
            self.env = Some(env);
        }
        if let Some(env_decode) = other.env_decode {
            self.env_decode = Some(env_decode);
        }
        if let Some(doc) = other.doc {
            self.doc = Some(doc);
        }
        if let Some(example) = other.example {
            self.example = Some(example);
        }
        if let Some(deprecated) = other.deprecated {
            self.deprecated = Some(deprecated);
        }
        self.has_default |= other.has_default;
        if other.merge_explicit {
            self.merge = other.merge;
            self.merge_explicit = true;
        }
        if let Some(allowed_sources) = other.allowed_sources {
            self.allowed_sources = Some(allowed_sources);
        }
        if let Some(denied_sources) = other.denied_sources {
            self.denied_sources = Some(denied_sources);
        }
        for rule in other.validations {
            self.upsert_validation(rule);
        }
        for (rule_code, config) in other.validation_configs {
            self.validation_configs.insert(rule_code, config);
        }
    }

    pub(in crate::metadata) fn is_env_decoder_only(&self) -> bool {
        self.env_decode.is_some()
            && self.aliases.is_empty()
            && !self.secret
            && self.env.is_none()
            && self.doc.is_none()
            && self.example.is_none()
            && self.deprecated.is_none()
            && !self.has_default
            && !self.merge_explicit
            && self.allowed_sources.is_none()
            && self.denied_sources.is_none()
            && self.validations.is_empty()
            && self.validation_configs.is_empty()
    }
}
