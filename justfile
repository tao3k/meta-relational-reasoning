set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

profile := "./.devenv/devenv-profile-exec"

default:
    @just --list

# Prefetch immutable upstream objects before the package manager builds them.
deps:
    {{profile}} python3 tools/ci/prepare-gerbil-dependencies.py
    {{profile}} mrr-gerbil deps --install

# Build the Gerbil package through the SDK-sanitizing repository wrapper first.
build:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo build --workspace --locked

# Run the complete local contract suite in canonical dependency order.
test:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py self-test
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py scheme
    {{profile}} mrr-cargo test --workspace --locked

# Qualify native tests with real stage output and a five-second silence cutoff.
test-native:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo test -p mrr-gerbil --tests --locked --offline --no-run
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py self-test
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py scheme
    {{profile}} gerbil env python3 tools/check/native-tests.py rust

# Run the backend-neutral Search factor contracts independently.
test-search:
    {{profile}} mrr-cargo test -p mrr-search --locked
    {{profile}} mrr-cargo test -p mrr-asp-rust-build-support --locked

# Check explicit Context selection, source admission, shared C4 replay, and proofs.
test-agentic-ai-context:
    {{profile}} python3 tools/check/context-features.py
    {{profile}} gerbil env python3 tools/check/context-qualify.py matrix
    {{profile}} mrr-cargo build -p meta-relational-reasoning --examples --locked --features agentic-ai-context-tokens
    {{profile}} python3 tools/check/context-qualify.py workflow
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR && lake env lean --run Checks/Main.lean && lake env lean Checks/Axioms.lean'


# Compare production Rust worklists and exact CBOR/SHA-256 preimages with Lean.
context-refinement:
    {{profile}} mrr-cargo build -p meta-relational-reasoning --example context_refinement --no-default-features --features agentic-ai-context-tokens --locked
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR'
    {{profile}} gerbil env python3 tools/check/context-refinement.py

# Regenerate exact production functions before checking the extracted Lean laws.
context-source-proof toolchain_dir:
    {{profile}} python3 tools/check/context-source-proof.py --toolchain-dir {{toolchain_dir}}
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContextRust && lake build Evidence Driver && lake env lean Axioms.lean'

# Record time, peak RSS, bounded rejection and actual declared reuse eligibility.
context-scale:
    {{profile}} mrr-cargo build -p meta-relational-reasoning --example context_scale --locked --features agentic-ai-context-tokens
    {{profile}} python3 tools/check/context-scale.py

# Enforce Rust lints after refreshing the Gerbil native inputs.
lint:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo clippy --workspace --all-targets --locked -- -D warnings

# Validate executable proof and live-evidence projects.
evidence:
    env UV_CACHE_DIR=/tmp/mrr-uv-cache {{profile}} uv --project proofs/MRRProof run pytest -q proofs/MRRProof/tests
    env UV_CACHE_DIR=/tmp/mrr-uv-cache {{profile}} uv --project experiments/mrr-live run pytest -q experiments/mrr-live/tests

# Full local admission gate.
check: test lint evidence

# Remove generated Gerbil and Rust artifacts through their owning tools.
clean:
    {{profile}} mrr-gerbil clean
    {{profile}} mrr-cargo clean
