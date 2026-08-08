# gql-rust

An ISO/IEC 39075-first, backend-neutral GQL compiler frontend for Rust.

The `gql-core` crate is the dependency-pure language implementation. The `gql`
facade enables the optional Ascent derived-relation backend by default for an
ergonomic `GQL + relational reasoning` distribution:

```toml
gql = "0.1"
```

Consumers that need only the language implementation can either depend on
`gql-core` directly or disable facade defaults:

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

