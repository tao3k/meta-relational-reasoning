# gql-rust

An ISO/IEC 39075-first, backend-neutral GQL compiler frontend for Rust.

The `gql-core` crate is the dependency-pure language implementation. The `gql`
facade enables the optional Ascent derived-relation backend by default:

```toml
gql = "0.1"
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
the full standard. External implementations may inform developer research, but
they are not project dependencies, oracles, pinned identities, fixtures, or CI
inputs and never override repository-owned architecture or ISO clause evidence.

## Architecture gates

- ISO language authority flows from the standard, never from an oracle.
- Core crates must not depend on `ascent`; the legacy `gql-ascent` and
  `gql-reasoning` authorities do not exist.
- `mrr-ascent` consumes admitted MRR bundles and only proposes bounded closure
  candidates; publication remains owned by the MRR facade.
- Backend features must not add parser keywords.
- Derived relations carry authority, provider, ruleset, snapshot, and closure
  evidence.

## Verification

```bash
cargo test --workspace
cargo test -p gql
cargo tree -p gql-core
./.devenv/devenv-profile-exec uv --project proofs/MRRProof run \
  mrr-lean-validate crates/mrr-identity/src/api.rs
```

## Publish readiness policy

`cargo package` is currently excluded for workspace member crates because the workspace
contains local-only, unpublished dependency edges (including `mrr-rust-project-harness-policy`)
that are intentionally part of current developer-gating design. The CI workflow keeps this
policy explicit and skips publish dry-runs until a release topology that resolves these
cross-crate dependencies is in place.
