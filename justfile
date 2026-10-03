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
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py scheme
    {{profile}} mrr-cargo test --workspace --locked

# Qualify native tests with real stage output and a five-second silence cutoff.
test-native:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo test -p mrr-gerbil --tests --locked --offline --no-run
    {{profile}} mrr-gerbil env python3 tools/check/native-tests.py scheme
    {{profile}} gerbil env python3 tools/check/native-tests.py rust

# Run the backend-neutral Search factor contracts independently.
test-search:
    {{profile}} mrr-cargo test -p mrr-search --locked
    {{profile}} mrr-cargo test -p mrr-asp-rust-build-support --locked

# Check explicit Context selection, source admission, shared C4 replay, and proofs.
test-agentic-ai-context:
    {{profile}} python3 tools/check/context-features.py
    {{profile}} mrr-cargo test -p meta-relational-reasoning --lib --locked --no-default-features
    {{profile}} mrr-cargo test -p mrr-agentic-ai-context --locked --no-default-features
    {{profile}} mrr-cargo test -p mrr-agentic-ai-context --locked --no-default-features --features token-layout
    {{profile}} mrr-cargo test -p meta-relational-reasoning --lib --locked --no-default-features --features agentic-ai-context
    {{profile}} mrr-cargo test -p meta-relational-reasoning --lib --locked --no-default-features --features agentic-ai-context-tokens
    {{profile}} bash -c 'cd proofs/MRRProof/AgenticAIContext && lake build MRR && lake env lean --run Checks/Main.lean && lake env lean Checks/Axioms.lean'


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
