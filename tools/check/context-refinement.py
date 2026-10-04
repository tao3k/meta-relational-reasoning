#!/usr/bin/env python3
"""Qualify actual Rust receipts against independent executable Lean models."""

import argparse
import importlib.util
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", default="/tmp/mrr-context-refinement.json")
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location(
        "native_tests", Path(__file__).with_name("native-tests.py")
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    receipt = Path(args.receipt).resolve()
    receipt.unlink(missing_ok=True)
    status = module.qualify(["target/debug/examples/context_refinement", str(receipt)])
    if status != 0:
        return status
    return module.qualify(
        ["lake", "env", "lean", "--run", "Checks/Refinement.lean", str(receipt)],
        cwd="proofs/MRRProof/AgenticAIContext",
    )


if __name__ == "__main__":
    raise SystemExit(main())
