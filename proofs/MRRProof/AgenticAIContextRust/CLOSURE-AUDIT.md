# Context implementation closure audit

## Verified starting point

PR #9 head `a7450879af27d08ec1cafd1b43e9375d5ac6af52` was verified
on 2026-10-04: both push/PR runs passed Ubuntu native, macOS native,
Rust source proofs and shared model/C4 replay. Both live-model jobs were skipped.
This is evidence for that exact head, not for subsequent changes. Concurrent
Temporal ownership documentation at `5c4f963` was subsequently preserved by a
fast-forward; it does not supply new proof evidence.

## Obligation inventory

| Production obligation | Evidence | Boundary / next acceptance condition |
| --- | --- | --- |
| Required roots initialization | `native_closure_seed_exact` | Actual roots, required ids and temporal receipts; Usize budget. |
| Forward traversal, success and typed rejection | `native_required_closure_declared_total` | Actual complete function; finite declared source derived from input; explicit machine bounds and stdlib value models. |
| Successful aggregate coverage | `native_required_closure_wrapper_exact` | Admitted source; exact least closure and maximum evidence weakness rank. |
| Reverse old/new edge union and traversal | `native_reverse_index_exact`, `native_declared_impact_wrapper_exact` | Actual edge enumeration and entry restoration; explicit value models, source/changed-set and capacity premises. |
| Revision old/new identity union | `native_revision_source_union` | Actual production helper; exact key union and uniqueness; no map/source validity premise. |
| Revision selected intersection minus invalidation | `native_revision_reusable_exact` | Actual production helper; exact membership and uniqueness, arbitrary input duplicates; old-list Usize bound. Does not establish that its invalidated argument is correct. |
| Revision global binding and element change classification | Unproved | Translate actual comparison/equality path; prove exact changed membership from actual snapshots, query, contract, payloads and dependencies. No assumed equality oracle. |
| Complete revision assembly | Unproved | Connect actual changed construction to reverse impact and reusable helper, including global drift and removed/new dependencies. |
| Source indexing and state validation/publication | Unproved as a whole Rust source theorem | Prove actual validation, budgets, duplicate/generation/dependency refusals and published-state invariant, then connect to selection/closure/revision. Existing model proofs and Rust tests are narrower evidence. |
| Serialization and cryptographic implementation | Bounded Rust/Lean replay only | Universal encoder correspondence is separate from hash injectivity/collision assumptions. Exact-reference profile avoids treating equal digests as identical contents. |
| Unsafe stdlib storage and allocation | Unproved | Pinned source probe fails on nested mutable entry borrows and raw-pointer operations; trusted value models are explicit, not this implementation proof. |
| Compilation and whole serving pipeline | Unproved | Verified compilation, external tokenizer/provider premises and physical serving/cache behavior require their own evidence. No acceleration claim follows from these proofs. |

## Production projection refactor

The public revision comparison calls the two proved helpers. All set construction
still uses Rust BTreeSet; actual sorted output ordering is preserved. The source
proof's extensional list models establish membership and uniqueness, not sorted
physical enumeration. Existing feature tests and fresh finite replay check the
public result behavior separately.

The extraction gate adds only `BTreeSet.contains` to its explicit external
interface (seven types, seventeen functions). Its borrowed extensional membership
model is computable; the proof derives concrete FactId membership using actual
identity equality and the included Borrow blanket implementation. No generated
external-template axiom is imported. Unsafe implementation correctness remains
an open obligation.

## Next implementation sequence

1. Finish actual revision comparison translation, first isolating the binding
   comparison from shared-state borrow joins. The full-comparison diagnostic
   must retain failures and must never emit proof success for partial Lean.
2. Prove each actual equality/changed-construction effect and connect it to the
   already proved index, impact and reusable projections. Run the required
   source audit and source-extraction gate, then the existing Rust/replay gates.
3. Extract and prove state validation/publication, with actual rejection paths,
   then compose the complete admitted workflow theorem.
4. Address encoder implementation correspondence and record unavoidable hash,
   unsafe-library, compilation and external-provider assumptions separately.

Each row closes only when its named acceptance condition passes. Adding a
hand-written independent model, a retained extraction failure, or another finite
replay case does not close a universal production refinement obligation.

## Retained comparison diagnostics

The original adapter-signature blocker is avoided by direct production loops.
A direct-loop extraction before helper separation retains `Could not match the
contexts` at global binding comparison. Full comparison after helper separation
hits the unchanged 60s translation cap after successful Charon extraction.
An independent-boolean attempt also timed out and was reverted, preserving
original short-circuit binding evaluation. These failures do not discharge
comparison or assembly. Timeout reports now also hash their exact stage logs.
