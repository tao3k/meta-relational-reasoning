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

`Driver.lean` proves six additional universal laws about the extracted while
loop: termination under an adapter contract, preservation of its invariant,
exact least closure from a completed graph invariant, refinement via a one-pop
simulation, immediate stopping with the mutated terminal state, and total correctness instantiated with the existing shared graph
model. The driver theorem has no fuel bound and applies to all state types. The
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

`Expansion.lean` proves the actual `state::expand_identity`, called by forward
traversal after map lookup and selected-set insertion. Missing facts, invalid
facts and required incomplete facts stop with the exact typed error, preserving
remaining pending identities and coverage. Accepted facts append their exact
dependencies, aggregate coverage by maximum weakness rank and preserve the error
slot. The accepted case has the same explicit machine-size premise as extension.
The extracted driver also returns the exact mutated state immediately when an
adapter reports stop. These facts do not prove the preceding BTree lookup and
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
./.devenv/devenv-profile-exec bash -c 'cd proofs/MRRProof/AgenticAIContextRust && MATHLIB_NO_CACHE_ON_UPDATE=1 lake update && lake exe cache get && lake build Evidence Driver NativeFacts Stack Expansion && lake env lean Axioms.lean'
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
driver uses an explicit generic trait parameter. The proved generated modules
contain no undefined external-function stubs. This avoids implicit Miri sysroot
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

`AdvanceContract` and `GraphAdvanceRefinement` are explicit theorem premises.
They have not been proved for `ClosureTraversal::advance` or
`ImpactTraversal::advance`, the actual BTreeMap/BTreeSet adapters. The model
instantiation does not discharge those native obligations. In particular, the
revision adapter suppresses duplicate enqueues and needs an invariant over
processed identities, rather than treating every scheduled identity as processed.
The identity-to-model mapping and native fact evidence policy now have universal
proofs. Map lookup/insertion, binding the admitted Fact to that lookup, full adapter
stopping/refinement laws, reverse-edge construction and processed versus
scheduled invariants remain open. The latest full wrapper probe exposes 9 external
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
