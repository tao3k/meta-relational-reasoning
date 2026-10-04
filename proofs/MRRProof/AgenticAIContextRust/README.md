# Production Rust evidence proofs

The native Context closure calls `evidence::admit_evidence` and
`evidence::merge_completeness`. Charon extracts their actual compiler IR, and
Aeneas generates `Generated/Types.lean` and `Generated/Funs.lean`. These files
are machine output; do not edit them manually.

`Evidence.lean` proves seven universal laws directly about those generated
functions: exact acceptance policy, invalidity precedence, rejection of required
incomplete evidence, merge refinement to maximum weakness rank, commutativity,
associativity, and complete evidence as the identity. `Axioms.lean` audits their
transitive axiom dependencies. No finite Rust replay is used to prove these laws.

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
./.devenv/devenv-profile-exec bash -c 'cd proofs/MRRProof/AgenticAIContextRust && MATHLIB_NO_CACHE_ON_UPDATE=1 lake update && lake exe cache get && lake build Evidence && lake env lean Axioms.lean'
```

Use `--rustup-home` for an isolated compiler installation. `--update` deliberately
refreshes generated files; the default fails if fresh output differs. The CI job
regenerates from the checked-out source before checking proofs, and uploads an
extraction receipt with tool versions and output hashes. Extraction freshness
and theorem checking are separate gates.

The ordinary pinned compiler sysroot is selected explicitly. The two extracted
functions use only Boolean branches and enum patterns, with no standard-library
calls or external function obligations. This avoids an implicit Miri sysroot
fallback. It is not a claim about the ordinary sysroot's library implementation.

## Remaining boundary

Charon successfully extracted the production `compute_required_closure` and
`reverse_dependency_impact` call graphs. The pinned Aeneas translator failed with
an internal error in `SymbolicToPureTypes.translate_fun_sigs`, attributed to
`core::iter::traits::iterator::Iterator`. Reproduce that probe with
`--probe-worklists`; failures propagate and never produce a success receipt.
The Rust standard-library maps/sets, wrappers mapping validity to a Boolean,
whole-loop invariants, serialization libraries and compiler are still outside
these source-derived proofs. Existing total-correctness worklist proofs and
Rust/Lean replay remain separate evidence for those boundaries.

The source extraction tools and Lean kernel form the trusted verification
pipeline. These seven laws do not establish verified Rust compilation or
end-to-end Context-system refinement.
