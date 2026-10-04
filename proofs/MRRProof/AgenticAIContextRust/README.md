# Production Rust Context proofs

The native Context closure calls `evidence::admit_fact_evidence`, which reads the
actual Fact/context/validity/completeness and calls `evidence::admit_evidence`,
and calls `evidence::merge_completeness`. Forward closure and reverse revision impact both
call `worklist::run`, a shared generic traversal driver. Charon extracts their
actual compiler IR, and Aeneas generates `Generated/Types.lean` and
`Generated/Funs.lean`. These files are machine output after the documented ASCII
normalization; do not edit them manually. The extractor includes reachable MRR
identity, relation and revision definitions instead of keeping private fields opaque.

`Evidence.lean` proves seven universal laws directly about the generated evidence
functions: exact acceptance policy, invalidity precedence, rejection of required
incomplete evidence, merge refinement to maximum weakness rank, commutativity,
associativity, and complete evidence as the identity.

`Driver.lean` proves seven additional universal laws about the extracted while
loop: termination under an adapter contract, preservation of its invariant,
exact least closure from a completed graph invariant, refinement via a one-pop
simulation, immediate stopping with the mutated terminal state, and total correctness instantiated with the existing shared graph
model and the enqueue-time suppression model. The driver theorem has no fuel bound and applies to all state types. The
model is compiled from the same `Closure`, `Worklist`, and `Termination` sources
as the Lean 4.34 system project, rather than copied. Its foundation imports `Std`.

The invariant-based graph theorem allows either pop-time duplicate suppression
(forward closure) or enqueue-time suppression (reverse impact). A one-pop
simulation is the stronger optional scheduling condition. `Axioms.lean` audits
the theorems' transitive axiom dependencies. No finite Rust replay proves these
universal laws.

`NativeFacts.lean` closes the source fact and identity seams: exact native field
accessors, the actual validity comparison, acceptance/rejection of the actual
Fact, the 32-byte FactId representation, and equality refinement. A proof-only
injective numbering of the full identity bytes transports least dependency
closure to and from the existing Nat graph model, for arbitrary dependency graphs.
This numbering is not a hash, and assumes no SHA-256 collision resistance or
injectivity from canonical inputs. FactId equality uses Aeneas's explicit
byte-array library model; this is part of the library modeling trust boundary,
not a source proof of Rust's standard-library byte comparison implementation.
The generated tuple-newtype representation aliases identity domains to byte
arrays. These theorems concern FactIds only and do not establish separation of
all MRR identity domains within Lean.

`Stack.lean` proves the actual source `worklist::pop_identity` and
`worklist::append_identities`: empty pop, total nonempty pop, last-in-first-out
behavior, preservation of extension order/duplicates, and both operations'
projection to the existing graph model's reversed pending list. The pop uses
checked subtraction, indexing and shrinking resize on Copy FactIds; it allocates
nothing and does not invoke the fill-value clone. Extension has an explicit
machine-size premise. These laws use Aeneas's existing Vec/Slice models; native
allocation failure and source proof of stdlib primitives remain outside them.
Neither operation is now an undefined external stub in the native traversal probe.
The actual `initial_pending` call assembles query roots, mandatory dependencies
and temporal receipts. Three further source laws preserve their exact order and
duplicates, project the complete combined root list, and establish the graph
initial invariant. They retain an explicit total machine-size bound and the
same Vec/Slice modeling boundary. Source extraction and the audit include these
initialization laws; the initialization slice brought the audit to 49 declarations.
The forward slice below brings the current required audit to 59.

`Expansion.lean` proves the actual `state::expand_identity`, called by forward
traversal after map lookup and selected-set insertion. Missing facts, invalid
facts and required incomplete facts stop with the exact typed error, preserving
remaining pending identities and coverage. Accepted facts append their exact
dependencies, aggregate coverage by maximum weakness rank and preserve the error
slot. The accepted case has the same explicit machine-size premise as extension.
The extracted driver also returns the exact mutated state immediately when an
adapter reports stop. These helper facts alone do not prove the preceding BTree lookup and
insertion, nor the complete native adapter contract.

## Reproduction

- Aeneas revision: `557eff83ecef5083b98a52a94ca7fae63d6c1dab`.
- Official release: `nightly-2026.10.03-557eff8`.
- Charon revision: `c8f15d7d658c86a95658f71ad99cddd4be002e04`.
- Rust: `nightly-2026-09-17`, with `rustc-dev` and `rust-src`.
- Lean: `4.31.0`; dependencies are locked in `lake-manifest.json`.

Download the official release for your platform and verify its asset checksum.
Install the pinned Rust toolchain. Then run through the repository profile:

```sh
./.devenv/devenv-profile-exec python3 tools/check/context-source-proof.py --toolchain-dir /path/to/aeneas
./.devenv/devenv-profile-exec bash -c 'cd proofs/MRRProof/AgenticAIContextRust && MATHLIB_NO_CACHE_ON_UPDATE=1 lake update && lake exe cache get && lake build Evidence Driver NativeFacts Stack Expansion Forward Reverse ReverseIndex ImpactWrapper Coverage Rejection ForwardWrapper GeneralForward GeneralWrapper DeclaredForward RevisionProjection BindingEquality && lake env lean Axioms.lean'
```

Use `--rustup-home` for an isolated compiler installation. `--update` deliberately
refreshes proved generated files; the default fails if fresh output differs. CI
regenerates from the checked-out source before checking proofs, and uploads an
extraction receipt with tool versions and output hashes. Extraction freshness
and theorem checking are separate gates.

The repository requires ASCII-only source. The extraction script replaces arrows
with Lean's native ASCII spellings and gives `Prod` a local `**` notation at the
same precedence as product notation. The Lean kernel checks the normalized code,
and fresh extraction compares those exact bytes. Unknown Unicode fails closed.
Handwritten proofs also use ASCII syntax; no policy exception is introduced.

The ordinary pinned compiler sysroot is explicitly selected with `--sysroot
default`. The evidence functions use Boolean branches and enum patterns; the
driver uses an explicit generic trait parameter. The proved generated modules use the explicitly allowlisted library models
and contain no imported external-template axioms. This avoids implicit Miri sysroot
fallback and makes no claim about the ordinary sysroot's library implementation.

## Remaining boundary

The earlier full-call-graph probe failed on `Iterator::copied`'s associated-type
constraint. Equivalent explicit stack initialization removed that first failure.
Extracting whole container loops then made no bounded progress. Separating native
one-pop adapters from the shared generic driver allowed the larger wrapper call
graph to translate too, but it emits external library obligations.

Run `--probe-worklists` to retain the actual LLBC and generated diagnostic modules
in a fresh temporary directory. The probe deliberately returns failure and
emits no success receipt: external-function/type stubs have not been discharged.
It cannot update the proved modules. Translation has a 60-second preparation cap.
These diagnostic modules are not imported into the proved project.

`Forward.lean` now proves the actual extracted `advance_closure`, its trait
instance, and `run_closure` under explicit standard-library value models. Its
`native_forward_advance_contract` discharges the forward adapter contract;
`native_forward_run_exact` proves termination without fuel, no error, and exact
selected membership in the least required closure. Preconditions are a finite
closed source graph, admitted facts agreeing with declared dependencies, the
initial graph invariant, and a Usize capacity bound. Neither an assumed adapter
step law nor an assumed stopping law is a premise.

The generated forward surface exposes exactly two BTree types and three library
functions. `Generated/TypesExternal.lean` and `Generated/FunsExternal.lean` supply
computable, trusted models for extensional lookup/membership, insertion, and array
comparison. The extraction gate rejects any different external interface and
records these handwritten model hashes. No generated template axioms are imported.
The models assume lawful key comparison and successful allocation; they do not
establish sorted iteration, BTree balancing, unsafe memory correctness, or the
Rust standard-library implementation. Axiom auditing checks Lean dependencies,
not correspondence of these models to unsafe Rust code.

The earlier reverse-model slice established scheduling at the graph level. In particular, the
revision adapter suppresses duplicate enqueues and needs an invariant over
processed identities, rather than treating every scheduled identity as processed.
`Scheduled.lean` now proves that invariant at the graph-model level: scheduled
membership equals processed-or-pending coverage, the pending list is duplicate
free and disjoint from processed, each nonempty step decreases finite capacity,
and a completed scheduled set is the exact least closure. Distinct initial roots
and neighbor lists express the native BTreeSet projections; they are explicit
premises of this model theorem. The extracted driver is instantiated with this
model and has no fuel bound. Both Lean projects compile the same model file.
The native traversal binding is now proved by the reverse slice below under explicit
library enumeration models; source proof of reverse-index union remains open.
The identity-to-model mapping and native fact evidence policy now have universal
proofs. Forward lookup/insertion and admitted-fact binding are now evaluated by the named
models. Native reverse-edge construction,
wrapper initialization/projection, whole-run coverage aggregation, rejection
integration, serialization refinement, and unsafe stdlib correctness remain open. An earlier full wrapper probe exposed 9 external
types and 16 external functions: BTreeMap/BTreeSet and their entry/iterator
interfaces, byte-array ordering and unused nonzero types in source-limit fields.
Vec pop/extension and the generic array/shared-Vec iteration stubs have been
removed by source-extractable operations and equivalent explicit traversal of
the old and new indexes. The blanket Borrow implementation is now extracted
from Rust's core source too. This inventory is a diagnostic, not a
proof of library semantics or native adapter contracts. The driver theorem covers
loop control once its step obligations hold. Existing Rust/Lean replay is separate
bounded evidence for the adapters.

The source extraction tools and Lean kernel form the trusted verification
pipeline, alongside the explicitly modeled standard-library primitives. These
laws do not establish verified Rust compilation,
serialization-library refinement or end-to-end Context-system refinement.

## Direct standard-library source boundary

`--probe-btree-source` attempts actual `alloc::collections::btree` source
extraction, including its unsafe node operations, starting from both native
wrappers. Each preparation stage has a 60-second cap. It retains LLBC and raw
stage logs in a fresh temporary directory, with tool versions, exact commands,
exit codes and log hashes in `library-source-probe.json`. That diagnostic always
has `proven: false`; it cannot update proved modules or emit a success receipt.

On the pinned compiler/extractor the direct probe fails: Charon reports node
allocation/deallocation translation limitations, and Aeneas rejects nested
mutable borrows in the entry API and a NonNull-to-raw-pointer transmute in
node.rs. This is an observed toolchain capability boundary, not a missing Lean
lemma. The explicit BTree models used by the forward proof are trusted library models;
they do not prove the unsafe Rust standard-library implementation. Complete
source refinement therefore remains unproved with the current toolchain.

Reproduce through the profile with the same pinned extractor arguments as above,
adding `--probe-btree-source`. The nonzero result is expected diagnostic failure,
not a passing proof or test.

## Forward source slice

The forward slice brought the audit to 59 declarations, including ten additional universal
container/projection/step/contract/run laws. This closes the native forward
adapter obligation relative to the explicit library models. The full-wrapper
probe remains diagnostic and the direct unsafe-library probe remains unproved.

## Actual native reverse traversal slice

`Reverse.lean` evaluates the actual `advance_impact` iterator loop and its native
trait instance. Twelve new universal laws prove exact enqueue effects, membership,
duplicate suppression, selected-set duplicate freedom, length conservation,
lookup, stopping, reachable-frontier preservation and a strict finite capacity
rank. `native_impact_advance_contract` derives the actual adapter contract;
`native_impact_run_exact` proves that actual `run_impact` terminates without fuel
and returns exactly the least reverse dependency closure. No assumed adapter
step or stopping law is supplied.

`ReverseSource` explicitly requires the source/adjacency graph correspondence,
closed finite source, duplicate-free neighbor projections and a twice-source
machine-capacity bound. `ReverseInvariant` retains pending coverage and reachable
frontier properties. The initial state must satisfy it; this theorem does not
prove construction of the old/new reverse-index union or wrapper initialization.
The invalidated set is the scheduled set; processing is tracked by frontier
coverage, not equated with membership in that set.

The current extraction gate admits three library types and five functions. The
new set iterator model enumerates its extensional value list exactly once and
stops at the empty suffix. It proves neither the unsafe implementation nor
sorted physical iteration. The required axiom audit now covers 71 declarations;
this is Lean dependency auditing relative to explicit models, not a Rust stdlib
implementation proof. Earlier native-reverse-contract-open statements are
superseded within this boundary only. Wrapper initialization/projection,
reverse-index construction, aggregate coverage/error integration and
serialization/cryptographic implementation refinement remain separate gaps.

## Actual reverse-index construction and traversal connection

`ReverseIndex.lean` proves the extracted `build_reverse_index`, its actual map
iteration, dependency iteration, entry/default update and mutable-borrow
restoration. Seventeen additional universal declarations establish exact old/new
declared-edge union and duplicate-free neighbor lists. The graph projection and
initial frontier invariant are derived, rather than supplied as adapter laws.
`native_declared_impact_run_exact` connects the actual index constructor to actual
`run_impact`: it terminates and returns exactly the reverse closure of the changed
identities, including cycles and duplicate edges. Its remaining input premises
are a duplicate-free changed list, matching seed-vector contents and a finite
machine-capacity bound. The actual wrapper's seed-vector construction and final
projection are not yet proved by this theorem.

The extraction gate now checks six library types and eleven functions. Explicit
value models additionally cover map enumeration, entry/default behavior and
borrow restoration; unsafe stdlib implementation, sorted physical enumeration
and allocation success remain outside the proof. The required axiom audit covers
88 declarations. Earlier reverse-index-construction-open statements are
superseded within this boundary. Wrapper initialization/projection, aggregate
coverage/error integration and serialization/cryptographic implementation
refinement remain open.

## Actual reverse seed wrapper, aggregate coverage and typed rejection

`ImpactWrapper.lean` proves the actual `declared_dependency_impact` helper called
by `reverse_dependency_impact`: construction, changed-set iteration into a Vec,
element clone, terminating traversal and return projection. The seed-content
premise is eliminated. Changed-set duplicate freedom and the finite Usize bound
remain explicit well-formedness/resource premises. The clone model invokes the
actual element clone and preserves failures; the length model checks machine
bounds. This does not prove how revision comparison constructs `changed` or the
State field projection, and it does not establish sorted physical enumeration.

`Coverage.lean` derives a coverage invariant for actual forward steps, including
duplicate pops, and lifts it through the actual terminating driver. Final coverage
rank is the maximum weakness rank of selected evidence; the same result has exact
least-required-closure membership and no error under `SourceAdmits`. Empty selected
state with Complete coverage satisfies the initial coverage invariant. The source
graph, source admission, initial graph invariant and machine budget remain the
forward theorem's explicit premises.

`Rejection.lean` proves actual pop/insert/lookup/expansion/driver stopping for a
fresh missing, invalidated or required-incomplete identity at the current pending
frontier. The exact typed error is retained, coverage is unchanged and the driver
does not perform another advance. The rejected identity remains in the internal
selected set exactly as Rust does. This is not a theorem about the first bad
identity after an arbitrary accepted prefix, nor the outer Result wrapper or
publication admission.

The required audit now covers 100 declarations; the extraction gate checks six
types and thirteen functions. Actual reverse seed initialization/projection,
aggregate admitted-forward coverage and frontier rejection/driver integration
are closed within the named value models. Remaining gaps include forward outer
wrapper initialization/Result/collection, revision change/reuse construction,
global rejection-path characterization, serialization/cryptographic
implementation and unsafe stdlib correctness. Handwritten value models are
explicit assumptions, not a substitute for those implementation proofs.

## Actual forward outer wrapper and general typed rejection

`ForwardWrapper.lean` proves actual seed initialization, owned-set iteration and
Vec collection, then connects actual `compute_required_closure` to the native
closure/coverage theorem. It returns an actual typed Ok with exact closure
membership and the selected evidence maximum rank. Initial traversal and collect
laws are derived, not supplied. Its admitted-source and capacity premises remain
explicit; sorted physical collection and allocator implementation are not proved.

`GeneralForward.lean` permits missing, invalid and required-incomplete evidence,
without `SourceAdmits`. A strict finite rank covers every accepted/duplicate
prefix and proves actual driver termination. A returned error has a concrete
missing/invalid/incomplete witness in the original input; a successful stop has
exact least-closure membership and duplicate-free identities. This is sound error
classification, not a trace theorem naming the first rejected identity.
`GeneralWrapper.lean` carries both cases through the actual outer typed Result:
error returns do not collect selected identities, successful returns collect the
actual final set. `DeclaredForward.lean` derives graph correspondence and finite
source closure from actual root/required/temporal identities and declared element
dependencies. `native_required_closure_declared_total` therefore needs only the
explicit initial worklist and output Usize bounds, with no graph-correspondence,
source-admission, initialized-traversal or adapter-law premise.

The required audit now covers 116 declarations. Extraction allowlists seven
library types and sixteen functions, including owned enumeration. The pinned
Aeneas `-filter-trait-methods` option aligns generated Iterator instances with the
target model's fields; actual `collect.default` remains a modeled call, and its
complete value effect is proved. Fresh receipts record this configuration and
model hashes. Trusted BTree/Vec/iterator models do not prove unsafe stdlib, sorted
physical iteration, allocation or verified compilation. Earlier forward outer
wrapper and arbitrary-prefix rejection gaps are superseded within this boundary.

## Revision implementation diagnostic

`--probe-revision` starts from actual `compare_revisions`, now called by the
unchanged public comparison paths. Its body is moved unchanged to a private free
function because Charon cannot start directly from an inherent impl. Algorithm,
iteration order and public API remain unchanged. The probe cannot update proved
modules or emit a success receipt. It retains stages and hashes even when
translation fails. The pinned Aeneas fails during Iterator method signature
translation (`SymbolicToPureTypes.translate_fun_sigs`, lines 1000-1001), reporting
an internal error at Rust core Iterator, lines 42:0-42:24. The failing
assertion requires empty method-binder `trait_type_constraints`; actual iterator
adapters retain such constraints. This is a translator limitation, not a missing
Lean theorem. Revision change/reuse
construction remains unproved; the new entry and diagnostic are not closure of
that gate. Serialization/hash implementation and unsafe stdlib/compilation
correctness also remain separate gates.

A separate retained `--monomorphize-mut=all` revision diagnostic also has stage
exits 0/2 at the same Iterator signature assertion; it is not a successful
translation or implementation proof.

## Actual revision identity and reuse projections

`RevisionProjection.lean` proves two helpers called by actual production
`compare_revisions`: `revision_source_ids` builds exactly the old/new map-key
union without duplicates; `revision_reusable_ids` returns exactly the old/new
selected intersection excluding invalidated ids, without duplicates, even when
input selection lists contain duplicates. The latter requires the explicit old
selection Usize bound. Actual Rust BTreeSet ordering is preserved by the refactor;
this source theorem proves membership and uniqueness within extensional models,
not sorted unsafe enumeration or correctness of its invalidated argument.

Nine additional audited laws derive borrowed FactId membership, finite iterator
termination, exact insertion and filtering effects, and the two outer helpers.
The projection step used seven external types and seventeen functions, adding
only the computable borrowed BTreeSet membership model; the binding step below
extends that interface. No independent
hand-written replacement of production comparison, assumed adapter law, fuel,
or generated template axiom is used.

The old Iterator method-binder failure is avoided by explicit production loops.
`BindingEquality.lean` proves sixteen additional audited laws for the actual
production `revision_bindings_changed` helper. Primitive identities, complete
external revision and snapshot fields, ordered query roots, and every contract
field have exact derived equality laws. The final result is exactly forced
change or unequal snapshot/query/contract, without an equality oracle premise.
Snapshot digest comparison is exact byte comparison; digest computation and
collision freedom are separate obligations. The explicit String value equality
model brings the strict external interface to seven types/eighteen functions.

The production helper preserves the original short-circuit binding evaluation.
Full actual `compare_revisions` extraction now succeeds (Charon 0, Aeneas 0),
superseding the earlier context/timeout blockers for extraction. Diagnostic mode
still emits no proof success. Actual Value equality uses direct variant and
ordered-field recursion, retaining exact textual values and duplicate/order
semantics; an independent derived-equality regression oracle checks finite
samples. The pinned `-loops-to-rec` extraction option avoids recursive loop
monotonicity failures. Complete isolated Lean compilation passes with explicit
computable NonZero and library candidates outside the admitted default interface.
No generated code is patched and no equality axiom is admitted. Universal
recursive equality, exact changed construction and revision assembly remain open.
All 141 required declarations are checked for
custom axioms by `Axioms.lean`.

See [CLOSURE-AUDIT.md](CLOSURE-AUDIT.md) for the obligation inventory, exact green
starting head, remaining trust boundaries and acceptance conditions. Complete
change classification, revision assembly, state validation/publication,
serialization/hash, unsafe stdlib and compilation/provider obligations remain
separate gates.

The complementary [TLA+/Lean gate](../AgenticAIContextTLA/README.md) checks phase
ordering, weak-fair termination, complete invalidation before publication, and
real fault counterexamples. Actual exported TLC states are checked against Lean
publication/closure/reuse definitions. This cross-model conformance is distinct
from the universal production-source obligations above.
