#!/usr/bin/env python3
"""Check Context use interleavings and replay every finite input against Rust."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

from . import process_runner, quint_cases

ROOT = quint_cases.ROOT
FIXTURE = ROOT / "fixtures/agentic-ai-context/use.json"
MODEL = quint_cases.MODELS / "ContextUse.qnt"
CHECKS = ["source", "query", "contract", "observation", "authorization", "expiry"]


def inputs() -> list[list[int]]:
    data = json.loads(FIXTURE.read_text())
    if data.get("schema") != "mrr.context-use-cases.v1" or data.get("checks") != CHECKS:
        raise ValueError("unknown Context use fixture")
    cases = data["cases"]
    values = [case["valid"] for case in cases]
    expected = {tuple(i for i in range(6) if mask & (1 << i)) for mask in range(64)}
    if len(values) != 64 or {tuple(v) for v in values} != expected:
        raise ValueError("fixture must contain every finite check assignment exactly once")
    if any(case["accepted"] != (len(case["valid"]) == 6) for case in cases):
        raise ValueError("fixture admits an unsafe use")
    return values


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    values = inputs()
    sources = [MODEL, FIXTURE,
        ROOT / "crates/meta-relational-reasoning/src/agentic_ai_context/use_gate.rs",
        ROOT / "crates/meta-relational-reasoning/src/agentic_ai_context/admission.rs",
        ROOT / "crates/meta-relational-reasoning/src/agentic_ai_context/manifest.rs",
        ROOT / "crates/meta-relational-reasoning/src/agentic_ai_context/tokens.rs",
        ROOT / "crates/mrr-agentic-ai-context/src/state.rs",
        ROOT / "crates/meta-relational-reasoning/tests/unit/agentic_ai_context_use.rs",
        ROOT / "Cargo.lock", Path(__file__)]
    source_hashes = {str(p.relative_to(ROOT)): quint_cases.sha256(p.read_bytes()) for p in sources}
    quint = Path(os.environ.get("MRR_QUINT_BIN", "quint"))
    version = subprocess.run([str(quint), "--version"], capture_output=True,
                             text=True, check=True, timeout=5).stdout.strip()
    if version != "0.33.0":
        parser.error(f"expected Quint 0.33.0, found {version!r}")
    rust_log = receipt.parent / "context-use-rust.log"
    print("CONTEXT-USE-RUST: replay shared finite inputs and boundary regressions", flush=True)
    rust = process_runner.run([
        "cargo", "test", "-p", "meta-relational-reasoning", "--no-default-features",
        "--features", "agentic-ai-context-tokens", "--lib", "tests::agentic_ai_context_use::",
        "--locked", "--", "--nocapture",
    ], cwd=ROOT, log=rust_log, total_limit=900, label="CONTEXT-USE-RUST")
    if rust.status != 0 or b"3 passed; 0 failed" not in rust.output:
        print("CONTEXT-USE-FAIL: Rust boundary replay did not pass", flush=True)
        return 1
    results = []
    # Safe, all six omitted-check controls, and omission of a Rust-replayed input.
    checks = [("safe", -1, False)] + [(name, i, False) for i, name in enumerate(CHECKS)]
    checks.append(("replay-omission", -1, True))
    with tempfile.TemporaryDirectory(prefix="mrr-context-use-", dir=receipt.parent) as root:
        work = Path(root)
        config = work / "tlc-config.json"
        config.write_text('{"workers":"1","maxHeap":"-Xmx1G"}\n')
        for name, skip, omit in checks:
            replayed = values[1:] if omit else values
            sets = ", ".join("Set(" + ", ".join(map(str, v)) + ")" for v in replayed)
            source = work / "ContextUseCase.qnt"
            source.write_text(MODEL.read_text() + (
                f"\nmodule ContextUseCase {{\n  import ContextUse(SKIP = {skip}, "
                f"REPLAYED = Set({sets})).*\n}}\n"))
            log = receipt.parent / f"context-use-{name}.log"
            print(f"CONTEXT-USE-QUINT: {name}", flush=True)
            checked = process_runner.run([
                str(quint), "verify", source.name, "--main", "ContextUseCase",
                "--backend", "tlc", "--apalache-version", "0.62.1",
                "--server-endpoint", "127.0.0.1:8822", "--invariant", "safety",
                "--tlc-config", str(config), "--verbosity", "3",
            ], cwd=work, log=log, label="CONTEXT-USE-QUINT")
            output = checked.output.decode(errors="replace")
            try:
                generated, distinct, remaining = quint_cases.checker_counts(output)
                if name == "safe":
                    if not (checked.status == 0 and "[ok] No violation found" in output
                            and distinct == 128 and remaining == 0):
                        raise ValueError("safe model did not exhaust 128 states")
                elif not (checked.status == 1 and "Error: Invariant q_inv is violated." in output
                          and "error: found a counterexample" in output):
                    raise ValueError("required negative control did not produce a counterexample")
            except ValueError as error:
                print(f"CONTEXT-USE-FAIL: {name}: {error}", flush=True)
                return 1
            results.append({"case": name, "skip": skip, "exit": checked.status,
                "generated": generated, "distinct": distinct, "remaining": remaining,
                "instance_sha256": quint_cases.sha256(source.read_bytes()),
                "log_sha256": quint_cases.sha256(checked.output), "log": str(log)})
            print(f"CONTEXT-USE-CASE-OK: {name} states={distinct}", flush=True)
    if source_hashes != {str(p.relative_to(ROOT)): quint_cases.sha256(p.read_bytes()) for p in sources}:
        print("CONTEXT-USE-FAIL: sources changed during qualification", flush=True)
        return 1
    receipt.write_text(json.dumps({
        "schema": "mrr.context-use.quint-rust.v1",
        "scope": "finite use predicates and Rust replay; authority authenticity and atomic runtime use remain premises",
        "quint": {"version": version, "backend": "tlc", "apalache": "0.62.1"},
        "sources": source_hashes,
        "rust": {"exit": rust.status, "finite_inputs": len(values), "tests": 3,
                 "log": str(rust_log), "log_sha256": quint_cases.sha256(rust.output)},
        "cases": results,
    }, indent=2) + "\n")
    print("CONTEXT-USE-OK: Rust 64 inputs, Quint 128 states, 7 negative controls", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
