#!/usr/bin/env python3
"""Run separate bounded scale cases and record per-process peak RSS."""

import argparse
import json
import os
from pathlib import Path
import resource
import subprocess
import sys

from . import native_tests

CASES = {
    "small": [256, 1024, 16, 4, 3],
    "large-results": [4096, 32768, 256, 64, 3],
    "high-fanout": [4096, 8192, 1, 2048, 3],
    "duplicate-heavy": [4096, 32768, 1, 32, 3],
}


def one_case(name: str, binary: str, output: Path) -> int:
    output.mkdir(parents=True, exist_ok=True)
    path = output / f"{name}.json"
    path.unlink(missing_ok=True)
    os.environ["MRR_CONTEXT_SCALE_RECEIPT"] = str(path.resolve())
    status = native_tests.qualify([binary, *map(str, CASES[name])])
    if status != 0:
        return status
    record = json.loads(path.read_text())
    # This worker launches exactly one measured binary; its child RSS is independent.
    rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    record["peak_rss_bytes"] = int(rss if sys.platform == "darwin" else rss * 1024)
    record["case"] = name
    record["build_profile"] = "cargo dev (workspace optimized + debuginfo)"
    record["platform"] = sys.platform
    if (
        record["budget_rejections"] != 5
        or record["semantic_reusable"] != record["selected"]
    ):
        raise RuntimeError("scale receipt failed required invariants")
    path.write_text(json.dumps(record, indent=2) + "\n")
    print(
        f"SCALE-MEMORY-OK: {name}: {record['peak_rss_bytes']} peak RSS bytes",
        flush=True,
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/examples/context_scale")
    parser.add_argument("--output", default=".ci/mrr-context-scale")
    parser.add_argument("--case", choices=CASES)
    args = parser.parse_args()
    output = Path(args.output)
    if args.case:
        return one_case(args.case, args.binary, output)
    for name in CASES:
        print(f"SCALE-CASE: {name}", flush=True)
        status = subprocess.call(
            [
                sys.executable,
                "-m",
                "mrr_proof_validation.context_scale",
                "--binary",
                args.binary,
                "--output",
                str(output),
                "--case",
                name,
            ]
        )
        if status != 0:
            return status
    records = [json.loads((output / f"{name}.json").read_text()) for name in CASES]
    (output / "summary.json").write_text(json.dumps(records, indent=2) + "\n")
    print(f"SCALE-QUALIFICATION-OK: {output / 'summary.json'}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
