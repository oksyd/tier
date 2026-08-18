set shell := ["bash", "-euo", "pipefail", "-c"]

patch:
  cargo release patch --no-publish --execute

publish:
  cargo publish --workspace

ci:
    cargo fmt --all -- --check
    cargo check --workspace --no-default-features --features json,watch,yaml
    cargo check --workspace --examples --no-default-features
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --doc --all-features
    cargo nextest run --workspace --all-features
    cargo nextest run --workspace --no-default-features --no-tests pass
    cargo nextest run --workspace --no-default-features --features clap
    cargo nextest run --workspace --no-default-features --features derive
    cargo nextest run --workspace --no-default-features --features schema
    cargo doc --workspace --no-deps --all-features
    RUSTDOCFLAGS='--cfg docsrs' cargo +nightly doc -p tier --all-features --no-deps

bench:
  cargo bench -p tier --bench core
  cargo bench -p tier --bench core --all-features
