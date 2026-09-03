# Meta-Relational Reasoning

A language-neutral Meta-Relational Reasoning (MRR) workspace for building
bounded, explainable reasoning systems with explicit identity, provenance,
lineage, transition, and admission boundaries.

The public Rust facade is `meta-relational-reasoning`. It composes the typed
MRR contracts and exposes query, deduction, explanation, impact analysis, and
atomic closure materialization without creating a second semantic authority.
Ascent is the core fixed-point engine: it proposes bounded derivations, while
the facade validates identities, lineage, snapshot transitions, budgets, and
complete admission before a result can be materialized.

Gerbil Scheme and POO own the declarative reasoning program, outer scheduling,
resource ordering, retry budgets, and termination. The declaration is compiled
ahead of time through `build.ss` into a fixed-width native ABI. Scheme does not
run inside the Rust/Ascent query hot path.

GQL and Cypher are frontend adapters in this workspace, not the identity of the
project. The ISO/IEC 39075 language profile remains evidence-driven and
non-certifying. Rowan is used only as a lossless CST sink; external graph
implementations such as SeleneDB, Grafeo, and froGQL are non-normative research
references rather than dependencies or semantic authorities.

## Architecture

- `meta-relational-reasoning` is the stable consumer facade.
- `mrr-identity`, `mrr-relation`, `mrr-query`, and `mrr-logic` define typed
  semantic contracts.
- `mrr-bundle` admits complete reasoning bundles.
- `mrr-ascent` computes bounded fixed-point candidates.
- `mrr-lineage` and `mrr-transition` remain the sole lineage and snapshot-delta
  validators.
- `mrr-gerbil` consumes the Scheme AOT projection and exposes its typed native
  boundary.
- `mrr-conformance` exercises the public facade across multiple domains.
- `gql-core` and `gql` implement the ISO GQL frontend adapter.
- `experiments/mrr-live` evaluates real model proposals while keeping Scheme
  scheduling and MRR receipts authoritative.

The project does not provide legacy compatibility modes or alternate admission
paths. Provider output is observational input; it cannot publish facts, assign
semantic authority, or replace an MRR receipt.

## Verification

```bash
./.devenv/devenv-profile-exec env \
  GERBIL_PATH="$PWD/.gerbil" gxi build.ss compile
./.devenv/devenv-profile-exec cargo test --workspace
./.devenv/devenv-profile-exec cargo test -p mrr-conformance --all-targets
./.devenv/devenv-profile-exec env GQL_HARNESS_VERIFY=1 \
  cargo check --workspace --all-targets
./.devenv/devenv-profile-exec uv --project proofs/MRRProof run pytest -q \
  proofs/MRRProof/tests
./.devenv/devenv-profile-exec uv --project experiments/mrr-live run pytest -q \
  experiments/mrr-live/tests
```

## Status

All workspace crates currently share the `0.1` release line. The repository is
still pre-release research and engineering work: executable conformance gates
are evidence for implemented contracts, not a claim of full ISO certification
or general-purpose reasoning completeness.

`cargo package` is intentionally excluded while workspace members retain
local-only unpublished dependency edges used by the current policy and proof
gates. Release packaging will be enabled only after those dependencies have a
closed publication topology.
