#!/usr/bin/env python3
"""Regenerate production evidence functions with pinned Charon and Aeneas."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


AENEAS_VERSION = "nightly-2026.10.03-557eff8"
CHARON_REVISION = "c8f15d7d658c86a95658f71ad99cddd4be002e04"
ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / "proofs/MRRProof/AgenticAIContextRust"


def run(command: list[str], environment: dict[str, str]) -> None:
    print("SOURCE-PREPARE:", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=environment, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain-dir", type=Path, required=True)
    parser.add_argument("--rustup-home", type=Path)
    parser.add_argument(
        "--receipt", type=Path, default=Path("/tmp/mrr-source-proof.json")
    )
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--probe-worklists", action="store_true")
    args = parser.parse_args()
    toolchain = args.toolchain_dir.resolve()
    environment = dict(os.environ)
    environment["PATH"] = f"{Path.home() / '.cargo/bin'}:{environment['PATH']}"
    if args.rustup_home:
        environment["RUSTUP_HOME"] = str(args.rustup_home.resolve())
    aeneas = toolchain / "aeneas"
    charon = toolchain / "charon"
    version = subprocess.check_output([aeneas, "-version"], env=environment, text=True)
    charon_version = subprocess.check_output(
        [charon, "version"], env=environment, text=True
    )
    if AENEAS_VERSION not in version or CHARON_REVISION not in charon_version:
        raise RuntimeError("source proof requires the pinned extractor versions")
    sysroot = subprocess.check_output(
        [charon, "toolchain-path"], env=environment, text=True
    ).strip()
    # These two functions import no standard-library function bodies. Pin the
    # ordinary compiler sysroot explicitly, avoiding Miri setup/fallback drift.
    environment["CHARON_MIRI_SYSROOTS"] = sysroot
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    environment["CARGO_TARGET_DIR"] = str(receipt.parent / "mrr-source-proof-target")
    functions = (
        ["state::compute_required_closure", "state::reverse_dependency_impact"]
        if args.probe_worklists
        else ["evidence::admit_evidence", "evidence::merge_completeness"]
    )
    with tempfile.TemporaryDirectory(prefix="mrr-source-proof-") as directory:
        output = Path(directory)
        llbc = output / "mrr_agentic_ai_context.llbc"
        starts = [
            argument
            for name in functions
            for argument in ["--start-from", f"mrr_agentic_ai_context::{name}"]
        ]
        run(
            [
                str(charon),
                "cargo",
                "--preset=aeneas",
                *starts,
                "--dest-file",
                str(llbc),
                "--",
                "--package",
                "mrr-agentic-ai-context",
                "--lib",
                "--no-default-features",
                "--locked",
                "--offline",
            ],
            environment,
        )
        run(
            [
                str(aeneas),
                "-backend",
                "lean",
                "-dest",
                str(output),
                "-subdir",
                "Generated",
                "-namespace",
                "MRR.ContextRust",
                "-split-files",
                str(llbc),
            ],
            environment,
        )
        emitted = sorted((output / "Generated").glob("*.lean"))
        if {path.name for path in emitted} != {"Types.lean", "Funs.lean"}:
            raise RuntimeError(
                "unexpected extraction surface or external-definition obligations"
            )
        hashes = {}
        for path in emitted:
            content = path.read_bytes()
            hashes[path.name] = hashlib.sha256(content).hexdigest()
            target = PROJECT / "Generated" / path.name
            if args.update:
                target.write_bytes(content)
            elif target.read_bytes() != content:
                raise RuntimeError(f"stale source extraction: {target}")
        receipt.write_text(
            json.dumps(
                {
                    "schema": "mrr.context.source-extraction.v1",
                    "aeneas": version.strip(),
                    "charon": charon_version.strip(),
                    "functions": functions,
                    "llbc_sha256": hashlib.sha256(llbc.read_bytes()).hexdigest(),
                    "generated_sha256": hashes,
                    "boundary": "extraction freshness; Lean theorem and axiom checks are separate gates",
                },
                indent=2,
            )
            + "\n"
        )
    print("CONTEXT-SOURCE-EXTRACTION-OK", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        print(f"SOURCE-EXTRACTION-FAILED: child exit {error.returncode}", flush=True)
        raise SystemExit(error.returncode) from None
