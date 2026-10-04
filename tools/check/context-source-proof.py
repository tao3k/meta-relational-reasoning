#!/usr/bin/env python3
"""Regenerate production evidence, identity and traversal control from Rust."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


AENEAS_VERSION = "nightly-2026.10.03-557eff8"
CHARON_REVISION = "c8f15d7d658c86a95658f71ad99cddd4be002e04"
ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / "proofs/MRRProof/AgenticAIContextRust"


def ascii_lean(content: str) -> bytes:
    """Keep extraction reproducible while respecting repository ASCII policy.

    Arrows have native ASCII spellings. The sole added notation is a local ASCII
    spelling of Prod, with the same precedence as Lean's product notation. Unknown
    Unicode fails closed instead of being silently removed from generated code.
    """
    if "\u00d7" in content:
        content = content.replace(
            "namespace MRR.ContextRust\n",
            'local infixr:35 " ** " => Prod\n\nnamespace MRR.ContextRust\n',
            1,
        )
    content = content.translate(
        str.maketrans({"\u2192": "->", "\u2190": "<-", "\u00d7": "**"})
    )
    return content.encode("ascii")


def run(command: list[str], environment: dict[str, str], timeout: int = 300) -> None:
    print("SOURCE-PREPARE:", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=environment, check=True, timeout=timeout)


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
    if args.probe_worklists and args.update:
        parser.error("a native-adapter probe cannot replace proved generated modules")
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
    # The selected pure functions import no standard-library function bodies. Pin the
    # ordinary compiler sysroot explicitly, avoiding Miri setup/fallback drift.
    environment["CHARON_MIRI_SYSROOTS"] = sysroot
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    environment["CARGO_TARGET_DIR"] = str(receipt.parent / "mrr-source-proof-target")
    functions = (
        [
            "mrr_agentic_ai_context::state::compute_required_closure",
            "mrr_agentic_ai_context::state::reverse_dependency_impact",
        ]
        if args.probe_worklists
        else [
            "mrr_agentic_ai_context::evidence::admit_evidence",
            "mrr_agentic_ai_context::evidence::merge_completeness",
            "mrr_agentic_ai_context::worklist::run",
            "mrr_agentic_ai_context::evidence::admit_fact_evidence",
        ]
    )
    with tempfile.TemporaryDirectory(prefix="mrr-source-proof-") as directory:
        output = Path(directory)
        llbc = output / "mrr_agentic_ai_context.llbc"
        starts = [argument for name in functions for argument in ["--start-from", name]]
        run(
            [
                str(charon),
                "cargo",
                "--preset=aeneas",
                "--sysroot",
                "default",
                "--include",
                "mrr_identity",
                "--include",
                "mrr_relation",
                "--include",
                "mrr_revision",
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
            timeout=60,
        )
        emitted = sorted((output / "Generated").glob("*.lean"))
        if args.probe_worklists:
            diagnostic = Path(
                tempfile.mkdtemp(prefix="mrr-native-adapter-probe-", dir=receipt.parent)
            )
            shutil.copy2(llbc, diagnostic / llbc.name)
            shutil.copytree(output / "Generated", diagnostic / "Generated")
            obligations = {
                path.name: re.findall(
                    r"^axiom\s+([^\s({:]+)", path.read_text(), re.MULTILINE
                )
                for path in emitted
                if "External" in path.name
            }
            (diagnostic / "obligations.json").write_text(
                json.dumps(
                    {
                        "schema": "mrr.context.native-adapter-obligations.v1",
                        "proven": False,
                        "functions": functions,
                        "obligations": obligations,
                    },
                    indent=2,
                )
                + "\n"
            )
            print(f"SOURCE-PROBE-DIAGNOSTICS: {diagnostic}", flush=True)
            raise RuntimeError(
                "native adapter translation is diagnostic only; external obligations are not discharged"
            )
        if {path.name for path in emitted} != {"Types.lean", "Funs.lean"}:
            raise RuntimeError(
                "unexpected extraction surface or external-definition obligations"
            )
        hashes = {}
        for path in emitted:
            content = ascii_lean(path.read_text())
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
                    "generated_format": "ASCII arrows and local Prod notation; unknown Unicode rejected",
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
    except subprocess.TimeoutExpired:
        print("SOURCE-EXTRACTION-FAILED: stage timeout", flush=True)
        raise SystemExit(124) from None
    except RuntimeError as error:
        print(f"SOURCE-EXTRACTION-FAILED: {error}", flush=True)
        raise SystemExit(65) from None
