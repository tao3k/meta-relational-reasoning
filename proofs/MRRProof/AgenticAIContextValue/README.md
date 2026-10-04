# Actual Rust Value equality source proof

This scope extracts the production `equal_value_lists`, `equal_value_records`,
`equal_record_fields` and recursive `Value::eq` from `mrr-relation`. Charon and
Aeneas versions match the Context Rust source proof. Generated files are checked
byte for byte; only the documented ASCII conversion is applied.

It is separate from `AgenticAIContextRust` because the generated discriminant
instances share global names even when Aeneas namespaces differ. This is not a
hand-written replacement implementation or a proof of complete revision assembly.

## Admitted interface

There are no external type obligations. The sole generated external function is
exact String value equality, modeled as `.ok (decide (left = right))`. Aeneas's
existing array, byte, Vec, Slice and machine-integer value models remain trusted
interfaces. No Rust unsafe storage or allocation implementation is verified.
Generated external-template axioms are never imported.

## Current laws

The audit requires 29 declarations: 26 actual source branch/composition laws and
three concrete byte-array or generic list/Vec composition laws. Source laws cover
all twelve atomic variants, constructor mismatches, key short circuit, list/record
delegation, unequal lengths, empty inputs, exhausted loops and an already-false
loop accumulator. These quantify over all inputs in their stated domains.
`native_atomic_value_equality_exact` combines all atomic branches into exact
logical equality against any right-hand value, with no comparison oracle premise.
Its classical DecidableEq instance is proof-only and supplies no Rust implementation.

The full nonempty recursive list/record total-correctness theorem remains open.
Delegation and stop laws do not establish recursive equality correctness. The
changed-set construction and complete revision result are still separate gates.
TLA lifecycle checking and Lean publication proofs continue to guard scheduling
and publication; they do not discharge these Rust equality obligations.

## Verification

From the repository root, through the captured profile:

```sh
./.devenv/devenv-profile-exec python3 tools/check/context-source-proof.py \
  --value-equality --toolchain-dir /path/to/pinned/tools \
  --receipt /tmp/mrr-value-source-proof.json
./.devenv/devenv-profile-exec bash -c \
  'cd proofs/MRRProof/AgenticAIContextValue && lake build ValueEquality && lake env lean Axioms.lean'
```

`--update` regenerates the source modules. The normal gate refuses stale generated
files, unexpected extraction modules or any additional external interface. Lean
compilation and the required-declaration axiom audit are separate acceptance gates.
