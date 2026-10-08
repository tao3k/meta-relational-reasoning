set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

profile := "./.devenv/devenv-profile-exec"
proof_python := "env PYTHONPATH=proofs/MRRProof/src python3 -m"

default:
    @just --list

# Prefetch immutable upstream objects before the package manager builds them.
deps:
    {{profile}} {{proof_python}} mrr_proof_validation.prepare_gerbil_dependencies
    {{profile}} mrr-gerbil deps --install

# Build the Gerbil package through the SDK-sanitizing repository wrapper first.
build:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo build --workspace --locked

# Run the complete local contract suite in canonical dependency order.
test:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_prepare
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_tests self-test
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_tests scheme
    {{profile}} mrr-cargo test --workspace --locked

# Qualify native tests with real stage output and a five-second silence cutoff.
test-native:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo test -p mrr-gerbil --tests --locked --offline --no-run
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_prepare
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_tests self-test
    {{profile}} mrr-gerbil env {{proof_python}} mrr_proof_validation.native_tests scheme
    {{profile}} gerbil env {{proof_python}} mrr_proof_validation.native_tests rust

# Run the backend-neutral Search factor contracts independently.
test-search:
    {{profile}} mrr-cargo test -p mrr-search --locked
    {{profile}} mrr-cargo test -p mrr-asp-rust-build-support --locked

# Check explicit Context selection, source admission, shared C4 replay, and proofs.
test-agentic-ai-context:
    {{profile}} {{proof_python}} mrr_proof_validation.context_features
    {{profile}} gerbil env {{proof_python}} mrr_proof_validation.context_qualify matrix
    {{profile}} mrr-cargo build -p meta-relational-reasoning --examples --locked --features agentic-ai-context-tokens
    {{profile}} {{proof_python}} mrr_proof_validation.context_qualify workflow
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR && lake env lean --run Checks/Main.lean && lake env lean Checks/Axioms.lean'


# Compare production Rust worklists and exact CBOR/SHA-256 preimages with Lean.
context-refinement:
    {{profile}} mrr-cargo build -p meta-relational-reasoning --example context_refinement --no-default-features --features agentic-ai-context-tokens --locked
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR agentic-ai-context-refinement'
    {{profile}} gerbil env {{proof_python}} mrr_proof_validation.context_refinement

# Regenerate exact production functions before checking the extracted Lean laws.
context-source-proof toolchain_dir:
    {{profile}} {{proof_python}} mrr_proof_validation.context_source_proof --toolchain-dir {{toolchain_dir}}
    {{profile}} {{proof_python}} mrr_proof_validation.context_source_proof --value-equality --toolchain-dir {{toolchain_dir}} --receipt .ci/value-source-proof.json
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContextRust && lake build Evidence Driver NativeFacts Stack Expansion Forward Reverse ReverseIndex ImpactWrapper Coverage Rejection ForwardWrapper GeneralForward GeneralWrapper DeclaredForward RevisionProjection BindingEquality && lake env lean Axioms.lean'
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContextValue && lake build ValueEquality ValueLoops ValueRecursive && lake env lean Axioms.lean'

# Check finite revision interleavings, fairness and required negative controls.
context-quint receipt:
    {{profile}} npm ci --prefix proofs/quint
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR agentic-ai-context-quint-replay && lake env lean Checks/Axioms.lean'
    {{profile}} {{proof_python}} mrr_proof_validation.context_quint --lean-replay proofs/MRRProof/AgenticAIContext/.lake/build/bin/agentic-ai-context-quint-replay --receipt {{receipt}}

# Qualify the finite Meta Impact model and its required counterexamples.
meta-impact receipt:
    {{profile}} npm ci --prefix proofs/quint
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR && lake env lean Checks/Axioms.lean'
    {{profile}} {{proof_python}} mrr_proof_validation.meta_impact_quint --receipt {{receipt}}

# Record time, peak RSS, bounded rejection and actual declared reuse eligibility.
context-scale:
    {{profile}} mrr-cargo build -p meta-relational-reasoning --example context_scale --locked --features agentic-ai-context-tokens
    {{profile}} {{proof_python}} mrr_proof_validation.context_scale

# Enforce Rust lints after refreshing the Gerbil native inputs.
lint:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo clippy --workspace --all-targets --locked -- -D warnings

# Validate executable proof and live-evidence projects.
evidence:
    env UV_CACHE_DIR=.ci/uv-cache {{profile}} uv --project proofs/MRRProof run pytest -q proofs/MRRProof/tests
    env UV_CACHE_DIR=.ci/uv-cache {{profile}} uv --project experiments/mrr-live run pytest -q experiments/mrr-live/tests

# Full local admission gate.
check: test lint evidence

# Remove generated Gerbil and Rust artifacts through their owning tools.
clean:
    {{profile}} mrr-gerbil clean
    {{profile}} mrr-cargo clean
