set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

profile := "./.devenv/devenv-profile-exec"

default:
    @just --list

# Build the Gerbil package through the SDK-sanitizing repository wrapper first.
build:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-cargo build --workspace --locked

# Run the complete local contract suite in canonical dependency order.
test:
    {{profile}} mrr-gerbil build
    {{profile}} mrr-gerbil test
    {{profile}} mrr-cargo test --workspace --locked

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
