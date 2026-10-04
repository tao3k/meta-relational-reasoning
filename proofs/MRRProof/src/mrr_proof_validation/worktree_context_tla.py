#!/usr/bin/env python3
"""Qualify finite cross-WorkTree Context guards and required TLC mutants."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import tempfile

from . import native_tests
from .context_session_tla import Output, TLC_HASH

ROOT = Path(__file__).resolve().parents[4]
MODEL = ROOT / "proofs/MRRProof/AgenticAIContextTLA"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--tlc-jar", type=Path)
    args = parser.parse_args()
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    command = ["tlc"]
    jar_hash = None
    if args.tlc_jar:
        jar_hash = hashlib.sha256(args.tlc_jar.read_bytes()).hexdigest()
        if jar_hash != TLC_HASH:
            parser.error("TLC jar differs from pinned official v1.7.4")
        command = ["java", "-XX:+UseParallelGC", "-cp", str(args.tlc_jar.resolve()), "tlc2.TLC"]

    cases = [
        ("none", None),
        ("ignoreScope", "NoForeignRead"),
        ("ignoreReadAuth", "NoUnauthorizedRead"),
        ("ignoreSourceCut", "NoStaleSourceImport"),
        ("ignoreTargetCut", "NoStaleTargetImport"),
        ("ignoreCAS", "NoDoubleImport"),
        ("ignoreApplicable", "NoInapplicableImport"),
    ]
    template = (MODEL / "WorktreeContext.cfg").read_text()
    results = []
    with tempfile.TemporaryDirectory(prefix="mrr-worktree-context-", dir=receipt.parent) as directory:
        work = Path(directory)
        (work / "WorktreeContext.tla").write_bytes((MODEL / "WorktreeContext.tla").read_bytes())
        for bug, invariant in cases:
            config = template.replace('Bug = "none"', f'Bug = "{bug}"')
            if config == template and bug != "none":
                parser.error("Bug replacement failed")
            (work / f"{bug}.cfg").write_text(config)
            log = receipt.parent / f"worktree-context-{bug}.log"
            log.unlink(missing_ok=True)
            output = Output(log)
            print(f"WORKTREE-CONTEXT-TLA: {bug}", flush=True)
            status = native_tests.qualify(
                [*command, "-deadlock", "-workers", "2", "-seed", "1", "-fp", "0",
                 "-metadir", str(work / f"states-{bug}"), "-config", f"{bug}.cfg",
                 "WorktreeContext.tla"],
                output,
                cwd=directory,
            )
            content = output.data.decode(errors="replace")
            states = re.search(r"([\d,]+) states generated, ([\d,]+) distinct states", content)
            if states is None:
                print(f"WORKTREE-CONTEXT-TLA-FAILED: no state count for {bug}", flush=True)
                return 1
            if invariant is None:
                accepted = status == 0 and output.valid()
            else:
                accepted = status == 12 and f"Invariant {invariant} is violated." in content
            if not accepted:
                print(f"WORKTREE-CONTEXT-TLA-FAILED: {bug} exit {status}", flush=True)
                return 1
            results.append({
                "mutation": bug,
                "expected_invariant": invariant,
                "exit": status,
                "generated": int(states[1].replace(",", "")),
                "distinct": int(states[2].replace(",", "")),
                "config_sha256": hashlib.sha256(config.encode()).hexdigest(),
                "log": str(log),
                "log_sha256": hashlib.sha256(output.data).hexdigest(),
            })

    receipt.write_text(json.dumps({
        "schema": "mrr.context.worktree-tla.v1",
        "scope": "two WorkTrees, two Sessions, two workers, one transfer identity",
        "tlc": {"version": "2.19", "revision": "5a47802", "jar_sha256": jar_hash},
        "model_sha256": hashlib.sha256((MODEL / "WorktreeContext.tla").read_bytes()).hexdigest(),
        "cases": results,
    }, indent=2) + "\n")
    print(f"WORKTREE-CONTEXT-TLA-OK: {len(results)} cases", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
