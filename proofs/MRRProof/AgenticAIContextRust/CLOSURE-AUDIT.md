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
| Revision global binding classification | `native_revision_bindings_changed_exact` | Actual production helper and derived snapshot/query/contract equality; exact force or field inequality. String/Vec value models are explicit; digest equality is byte equality, not collision freedom. |
| Revision element change classification | Unproved | Prove actual recursive payload/element equality and exact changed membership from actual source maps. No assumed equality oracle. |
| Complete revision assembly | Unproved | Connect actual changed construction to reverse impact and reusable helper, including global drift and removed/new dependencies. |
| Source indexing and state validation/publication | Unproved as a whole Rust source theorem | Prove actual validation, budgets, duplicate/generation/dependency refusals and published-state invariant, then connect to selection/closure/revision. Existing model proofs and Rust tests are narrower evidence. |
| Serialization and cryptographic implementation | Bounded Rust/Lean replay only | Universal encoder correspondence is separate from hash injectivity/collision assumptions. Exact-reference profile avoids treating equal digests as identical contents. |
| Unsafe stdlib storage and allocation | Unproved | Pinned source probe fails on nested mutable entry borrows and raw-pointer operations; trusted value models are explicit, not this implementation proof. |
| Compilation and whole serving pipeline | Unproved | Verified compilation, external tokenizer/provider premises and physical serving/cache behavior require their own evidence. No acceleration claim follows from these proofs. |

## Production projection refactor

The public revision comparison calls the three proved helpers. All set construction
still uses Rust BTreeSet; actual sorted output ordering is preserved. The source
proof's extensional list models establish membership and uniqueness, not sorted
physical enumeration. Existing feature tests and fresh finite replay check the
public result behavior separately.

The extraction gate includes `BTreeSet.contains` and exact String value equality in its explicit
external interface (seven types, eighteen functions). Its borrowed extensional membership
model is computable; the proof derives concrete FactId membership using actual
identity equality and the included Borrow blanket implementation. No generated
external-template axiom is imported. Unsafe implementation correctness remains
an open obligation.

## Next implementation sequence

1. Complete Lean compilation of actual revision comparison, including its
   recursive payload equality. Full comparison now translates successfully;
   extraction alone is not a proof receipt.
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

The original adapter-signature and shared-state borrow-context blockers are
superseded for extraction by direct production loops and a binding helper that
preserves the original short-circuit evaluation. The retained final full
comparison diagnostic has Charon exit 0 and Aeneas exit 0. Diagnostic mode still
refuses to emit proof success; the generated complete function has not yet
passed Lean compilation or a complete refinement theorem.

The complete generated state includes two NonZero library types absent from
the default proved scope. A separate isolated compiler diagnostic supplies
computable nonzero-value candidates, retaining the nonzero constraint. These
candidates are not part of the admitted seven-type/eighteen-function interface.
With those isolated candidates, Lean reports unknown constant
`MRR.ContextRust.mrr_relation.api.Value.Insts.CoreCmpPartialEqValue` at
the generated recursive List/Record comparison branches: the function references
the trait instance before its later declaration. The pinned extractor does not
support mixed recursive function/trait groups. Generated source is not patched,
and no recursive equality axiom is admitted. Resolve this compiler obligation
before extending the default proof gate. Earlier context/timeout failures remain
historical diagnostics, not the current extraction blocker.

## Actual global binding equality

`BindingEquality.lean` adds sixteen required declarations, bringing the source
axiom audit to 141. It derives primitive identities, ordered vectors, external
revision identities, revision bindings, complete snapshot fields, query fields
and contract fields before proving the actual production binding helper. The
final theorem has no equality oracle premise. It exactly characterizes forced
change or unequal snapshot/query/contract; snapshot digest bytes are compared,
without asserting digest construction correctness or collision freedom.

The String external model is computable exact value equality. Vec equality
uses the pinned library value model and actual derived element comparators.
These proofs do not establish unsafe library storage/allocation, compilation,
or complete revision assembly. Complete element classification, changed-set
construction, state validation/publication and serialization remain open.
