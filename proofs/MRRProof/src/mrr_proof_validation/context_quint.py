#!/usr/bin/env python3
"""Qualify finite Quint revision lifecycle safety and liveness."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

from . import quint_cases, quint_runner


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    quint = Path(os.environ.get("MRR_QUINT_BIN", quint_cases.MODELS / "node_modules/.bin/quint")).resolve()
    version = subprocess.run([str(quint), "--version"], capture_output=True, text=True,
                             check=True, timeout=5).stdout.strip()
    if version != "0.33.0":
        parser.error(f"expected Quint 0.33.0, found {version!r}")

    results = []
    safe_cases = [(name, "none", None) for name in quint_cases.CASES["revision"]]
    mutant_cases = [(item["scenario"], item["bug"], item["invariant"])
                    for item in quint_cases.CASES["revisionMutations"]]
    with tempfile.TemporaryDirectory(prefix="mrr-context-quint-", dir=receipt.parent) as root:
        work = Path(root)
        config = work / "tlc-config.json"
        config.write_text('{"workers":"1","maxHeap":"-Xmx1G"}\n')
        for scenario, bug, invariant in safe_cases + mutant_cases:
            name = f"{scenario}-{bug}"
            source = quint_cases.instance("RevisionLifecycle", scenario, bug, work)
            log = receipt.parent / f"context-quint-{name}.log"
            print(f"CONTEXT-QUINT-QUALIFY: {name}", flush=True)
            command = [
                str(quint), "verify", source.name, "--main", "RevisionCase",
                "--backend", "tlc", "--apalache-version", "0.62.1",
                "--invariant", invariant or "safety",
                "--tlc-config", str(config), "--verbosity", "3",
            ]
            if invariant is None:
                command += ["--temporal", "eventuallyPublished,publishedStable"]
            checked = quint_runner.run(command, cwd=work, log=log)
            output = checked.output.decode(errors="replace")
            try:
                generated, distinct, remaining = quint_cases.checker_counts(output)
                if invariant is None:
                    if not (checked.status == 0 and "[ok] No violation found" in output
                            and "Finished checking temporal properties" in output
                            and remaining == 0):
                        raise ValueError("safe finite and temporal check did not complete")
                else:
                    if not (checked.status == 1 and "Error: Invariant q_inv is violated." in output
                            and "[violation] Found an issue" in output
                            and "error: found a counterexample" in output):
                        raise ValueError(f"mutation did not violate {invariant}")
            except ValueError as error:
                print(f"CONTEXT-QUINT-FAILED: {name}: {error}", flush=True)
                return 1
            print(f"CONTEXT-QUINT-OK {name} states={distinct}", flush=True)
            results.append({
                "scenario": scenario,
                "mutation": bug,
                "invariant": invariant or "safety+eventuallyPublished+publishedStable",
                "exit": checked.status,
                "generated": generated,
                "distinct": distinct,
                "remaining": remaining,
                "instance_sha256": quint_cases.sha256(source.read_bytes()),
                "log": str(log),
                "log_sha256": quint_cases.sha256(checked.output),
            })

    receipt.write_text(json.dumps({
        "schema": "mrr.context.quint-revision.v3",
        "quint": {"version": version, "backend": "tlc", "apalache": "0.62.1"},
        "scope": "finite four-identity lifecycle scenarios; not a Rust source proof",
        "models": {
            path.name: quint_cases.sha256(path.read_bytes())
            for path in (quint_cases.MODELS / "RevisionLifecycle.qnt", quint_cases.CASES_PATH)
        },
        "cases": results,
    }, indent=2) + "\n")
    print(f"CONTEXT-QUINT-OK: {len(results)} checked cases", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
