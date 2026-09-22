# tier-derive

`tier-derive` provides `#[derive(TierConfig)]` and `#[derive(TierPatch)]` for the
[`tier`](https://docs.rs/tier) Rust configuration library.

Use it when you want config metadata to stay next to your `serde` types instead
of being repeated in manual builder code. It derives env names, aliases,
examples, validation rules, merge behavior, and secret handling from the same
type definitions you already use for deserialization.

It generates structured `TierMetadata` implementations for nested config
structs, newtype wrappers, and serde-driven enums, and it can derive typed
sparse patch structs for CLI and last-layer overrides.

Supported metadata includes:

- field attributes such as `secret`, `env`, `doc`, `example`, `deprecated`,
  `merge`, and `leaf`
- declared validation such as `non_empty`, numeric and length bounds,
  `one_of`, hostname/IP/socket/path rules, and cross-field checks
- serde-aware naming and shape rules including `rename`, `rename_all`,
  `rename_all_fields`, `alias`, `default`, tagging, `skip`, and `flatten`
- automatic `tier::Secret<T>` detection
- typed patch mapping through `path`, `path_expr`, and `nested`

Most users should depend on `tier` with the `derive` feature enabled and use
the re-exported `tier::TierConfig` derive macro.

For internally tagged, adjacently tagged, and untagged enums, fields shared by
multiple variants keep their security metadata. A path marked secret in any
variant is redacted in all variants. Allowed source sets are intersected and
denied source sets are combined, including policies on nested fields; incompatible
combined policies produce a metadata error when loading. Variant-specific
validation rules on shared fields are omitted because they cannot safely apply
to every variant. Prefer distinct field paths when variants need different
security policies. Ambiguous aliases keep their security metadata without being
rewritten to a single variant's field path.

Variant-level `serde(rename_all)` overrides the enum's `rename_all_fields` for
that variant. Directional renames use deserialization names for configuration
paths and preserve serialization names as input aliases.
