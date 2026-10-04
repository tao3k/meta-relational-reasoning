#!/usr/bin/env python3
"""Qualify actual Rust receipts against independent executable Lean models."""

import argparse
from pathlib import Path

from . import native_tests


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", default=".ci/mrr-context-refinement.json")
    args = parser.parse_args()
    receipt = Path(args.receipt).resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    status = native_tests.qualify(["target/debug/examples/context_refinement", str(receipt)])
    if status != 0:
        return status
    return native_tests.qualify(
        [".lake/build/bin/agentic-ai-context-refinement", str(receipt)],
        cwd="proofs/MRRProof/AgenticAIContext",
    )


if __name__ == "__main__":
    raise SystemExit(main())
