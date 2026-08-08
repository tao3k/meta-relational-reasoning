# gql-rust

An ISO/IEC 39075-first, backend-neutral GQL compiler frontend for Rust.

The `gql-core` crate is the dependency-pure language implementation. The `gql`
facade can enable the optional Ascent derived-relation backend as a feature:

```toml
gql = { version = "0.1", features = ["ascent"] }
```

Consumers that need only the language implementation can either depend on
`gql-core` directly or keep `gql` defaults disabled:

```toml
gql = { version = "0.1", default-features = false }
```

Enabling Ascent never changes tokens, grammar, AST lowering, ISO types, or ISO
semantic rules. It only adds catalog and execution capability for externally
registered derived relations.

This repository is an early foundation, not yet a conforming implementation of
the full standard. Grafeo and froGQL may be used as differential oracles; neither
is a normative source or runtime dependency.

## Architecture gates

- ISO language authority flows from the standard, never from an oracle.
- Core crates must not depend on `ascent` or `gql-ascent`.
- Optional backends consume canonical IR and catalog contracts.
- Backend features must not add parser keywords.
- Derived relations carry authority, provider, ruleset, snapshot, and closure
  evidence.

## Verification

```bash
cargo test --workspace
cargo test -p gql --no-default-features
cargo test -p gql --all-features
cargo tree -p gql-core
```

## Publish readiness policy

`cargo package` is currently excluded for workspace member crates because the workspace
contains local-only, unpublished dependency edges (including `gql-rust-project-harness-policy`)
that are intentionally part of current developer-gating design. The CI workflow keeps this
policy explicit and skips publish dry-runs until a release topology that resolves these
cross-crate dependencies is in place.
