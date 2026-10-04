# TLA+ and Lean revision contract

This gate checks both formalizations on actual TLC output. TLA+ models revision
phases and all reachable interleavings of classification, invalidation, sealing,
and publication for each configured finite graph. Lean checks the exported
states against its existing revision closure and reuse definitions, and rejects
the actual mutation counterexamples at its publication guard.

## Shared obligations

| TLA+ expression | Lean definition / theorem | Actual Rust source evidence |
| --- | --- | --- |
| `Changed` and `Classification` | `checkRevisionChanges`; canonical values, conservative changed sets allowed | `native_revision_bindings_changed_exact` for global binding drift. Element equality/changed construction remains open. |
| `Edges = OldEdges \cup NewEdges` | `revisionDependencies`; `reverseDependencies` | `native_reverse_index_exact` and `native_declared_impact_wrapper_exact`, with explicit value models and source/capacity premises. |
| `Impact` and `CompleteBeforePublish` | `checkRequiredClosure`; `published_revision_impact_exact` | Complete actual revision assembly remains open. |
| `ReuseSafe` | `revisionReusable`; `published_revision_dependency_safe` | `native_revision_reusable_exact`; actual invalidated input correctness is separate. |
| `Seal` / `Publish` | `publishRevision`; `published_revision_gates` | Model publication gate, not complete Rust validation/publication proof. |
| `EventuallyPublished` | Existing finite-worklist termination laws cover traversal; no universal lifecycle fairness theorem is claimed | TLC checks weak fairness for each finite lifecycle scenario. This adds no physical-serving liveness guarantee. |

TLC identities are natural-number labels of finite semantic identities. Edges
point from users to dependencies. Dumped input variables record old/new edges,
selection, unequal values and global drift; all are frozen through the lifecycle.
The Lean replay consumes every distinct positive TLC state and the final actual
state of each invariant counterexample. Canonical old values are zero; new values
are one precisely for identities in `unequal`. This provides exact equality
status for the model without claiming equality of recursive production payloads.

Positive scenarios cover cycles, removed edges, added edges, diamonds, self
edges, global drift, unchanged values and different endpoint selections. Each
has four identities, with all enabled classification/impact orders explored.
Two model mutations must produce the named `CompleteBeforePublish` violation:
ignoring new edges and sealing before propagation completes. Their real TLC
states must also fail Lean's complete-impact publication guard. Unexpected
nonzero status, parse errors, missing states, missing counterexamples, or a
watchdog timeout fail the gate.

## Execution and evidence

```
./.devenv/devenv-profile-exec bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR agentic-ai-context-tla-replay && lake env lean Checks/Axioms.lean'
./.devenv/devenv-profile-exec python3 tools/check/context-tla.py --lean-replay proofs/MRRProof/AgenticAIContext/.lake/build/bin/agentic-ai-context-tla-replay --receipt /private/tmp/context-tla/receipt.json
```

The shared qualifier retains its five-second output-silence and 45-second batch
limits for each TLC run and the Lean replay. Actual solver/compiler preparation
and qualification remain separate. TLC needs a local RMI coordination socket;
a sandbox that prohibits local binding cannot run this gate without that access.
There are no synthetic progress heartbeats or timeout-based success paths.

CI pins the [official TLC v1.7.4 release](https://github.com/tlaplus/tlaplus/releases/tag/v1.7.4)
jar to SHA-256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`.
The receipt records exact tool identity, model/config/log hashes, generated and
distinct state counts, all-state projection hash, and Lean replay log hash.
Logs and projected states are saved alongside the receipt.

TLC finite model checking, universal Lean model laws, and actual Rust source
refinement are distinct evidence. This is executable conformance between two
models; it is not a mechanically verified TLA-to-Lean translator, a TLAPS proof
for arbitrary configurations, or a closed proof of the complete Rust pipeline.
Unsafe libraries, verified compilation and external providers remain explicit
boundaries in the Rust closure audit.
