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

## Why

A fixed-point engine can answer "what follows from these facts and rules?" It
does not, by itself, answer the operationally different question "what may now
become authoritative?" A candidate can be logically derivable and still be
unsafe to publish because its evaluation was truncated, its input generation is
stale, its evidence is incomplete, its identities collide, or its resulting
state transition is invalid.

MRR makes that publication boundary explicit:

1. Typed relations, rules, evidence, and immutable identities define the input.
2. Ascent computes bounded derivation candidates and a deterministic closure
   receipt. It does not allocate canonical fact or derivation identities.
3. The public facade admits only a complete receipt bound to the evaluated
   generation, a one-to-one set of unique caller-owned identities, valid
   lineage, and one valid semantic-generation transition.
4. The complete transition, derivations, and identity-bearing receipt are
   returned together. Any failed check publishes nothing.

This separation keeps parsers, model providers, graph query frontends, and
fixed-point evaluation useful without allowing any of them to become a second
source of semantic authority. It also preserves `unknown` and `incomplete` as
first-class outcomes instead of silently treating missing evidence as false.

## Comparison with adjacent systems

This is a comparison of design responsibilities, not a benchmark or a claim
that one system should replace another.

| System | Primary design center | Evidence or result | Relationship to MRR |
| --- | --- | --- | --- |
| MRR | Bounded reasoning followed by fail-closed publication | Generation-bound closure, lineage, transition, and identity-complete receipt | Owns the boundary between a proposed derivation and an authoritative semantic-generation delta |
| [Parlant](https://www.parlant.io/docs/quickstart/motivation/) | Conversational control through dynamically selected guidelines, journeys, glossary terms, tools, and response composition | A trace of the contextual items and actions used to produce a controlled response | Complementary: Parlant governs what a conversational agent should say or do; MRR governs whether derived facts and state changes may be admitted |
| [Ascent](https://s-arash.github.io/ascent/cc22main-p95-seamless-deductive-inference-via-macros.pdf) and [Souffle](https://github.com/souffle-lang/souffle) | Datalog-style fixed-point computation; Souffle also provides compiled parallel evaluation and provenance support | Derived relations and, where configured, provenance for logical results | MRR uses Ascent as its proposal engine, then adds identity allocation boundaries, bounded receipts, generation binding, and atomic admission |
| [Open Policy Agent](https://www.openpolicyagent.org/docs) | Domain-neutral policy evaluation over structured input and data | A policy decision, optionally accompanied by decision-log metadata | Complementary: OPA can decide policy; MRR validates and materializes a complete derivation and state-transition bundle |
| [TLA+ and TLC](https://lamport.azurewebsites.net/pubs/yuanyu-model-checking.pdf) | Specification and finite-state model checking of concurrent and reactive systems | Checked invariants or a model-level counterexample | Complementary: model checking validates a design space; MRR carries bounded transition evidence into typed runtime receipts and publication decisions |

The important distinction from Parlant is therefore not "rules versus no
rules." Both systems structure behavior and expose traces. Parlant's documented
engine selects relevant conversational guidance before composing an LLM
response. MRR's admission layer runs after candidate inference and accepts or
rejects an entire typed state delta. A deployment can use Parlant at the
interaction boundary and MRR behind it without merging their authority models.

Detailed design rationale lives with the component that owns each decision:

- [why Ascent is the bounded proposal engine](docs/architecture/0003-mrr-ascent-evaluation.org);
- [why semantic lineage is an admission contract](docs/architecture/0009-unified-lineage-v1.org);
- [why TLA+ and TLC are independent test oracles](docs/architecture/0021-mrr-differential-oracles.org).

### Evidence basis

- MRR's publication claims are executable contracts in
  [`admission.rs`](crates/meta-relational-reasoning/src/admission.rs), while
  [`truth.rs`](crates/meta-relational-reasoning/src/truth.rs) preserves the
  distinction between false, unknown, and incomplete.
- The proposal/admission split is explicit in
  [`mrr-ascent`](crates/mrr-ascent/src/api.rs): closure evaluation produces
  candidates that still await upstream identity allocation and lineage
  admission.
- The Parlant comparison is based on its official
  [motivation](https://www.parlant.io/docs/quickstart/motivation/) and
  [engine overview](https://www.parlant.io/docs/engine-internals/overview/),
  including guideline matching, tool calls, response composition, and traces.
- The logic-engine comparison uses the peer-reviewed
  [Ascent paper](https://s-arash.github.io/ascent/cc22main-p95-seamless-deductive-inference-via-macros.pdf)
  and Souffle's published
  [provenance work](https://souffle-lang.github.io/pdf/toplas20.pdf).
- The policy boundary follows OPA's official distinction between
  [policy decisions](https://www.openpolicyagent.org/docs) and
  [decision logs](https://www.openpolicyagent.org/docs/management-decision-logs).
- The model-checking boundary follows the original
  [TLC paper](https://lamport.azurewebsites.net/pubs/yuanyu-model-checking.pdf);
  this repository separately checks transition fixtures against TLC in
  [`oracle.rs`](crates/mrr-transition/tests/unit/oracle.rs).

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
- `mrr-frontends` lowers parser-owned ISO GQL artifacts directly into
  `MetaQueryIr` without a second Rust parser or public GQL AST stack.
- `experiments/mrr-live` evaluates real model proposals while keeping Scheme
  scheduling and MRR receipts authoritative.

The project does not provide legacy compatibility modes or alternate admission
paths. Provider output is observational input; it cannot publish facts, assign
semantic authority, or replace an MRR receipt.

## Verification

```bash
./.devenv/devenv-profile-exec mrr-gerbil build
./.devenv/devenv-profile-exec mrr-gerbil test
./.devenv/devenv-profile-exec mrr-cargo test --workspace
./.devenv/devenv-profile-exec mrr-cargo test -p mrr-conformance --all-targets
./.devenv/devenv-profile-exec env GQL_HARNESS_VERIFY=1 \
  mrr-cargo check --workspace --all-targets
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
