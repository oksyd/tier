mod array;
mod collector;
mod combinator;
mod entry;
mod object;
mod path;

use serde_json::Value;

use crate::docs::EnvDocEntry;

pub(super) fn collect_env_docs(schema: &Value, docs: &mut Vec<EnvDocEntry>) {
    collector::collect_env_docs(schema, docs);
}
