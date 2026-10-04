#!/usr/bin/env python3
"""Regenerate production evidence, identity and traversal control from Rust."""

import argparse
from contextlib import nullcontext
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
    diagnostic = environment.get("MRR_BTREE_SOURCE_PROBE_DIR")
    if diagnostic is None:
        subprocess.run(command, cwd=ROOT, env=environment, check=True, timeout=timeout)
        return
    directory = Path(diagnostic)
    report_path = directory / "library-source-probe.json"
    report = json.loads(report_path.read_text())
    stage = {"command": command, "proven": False}
    report["stages"].append(stage)
    log_path = directory / f"stage-{len(report['stages'])}.log"
    stage["log"] = str(log_path)
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    with log_path.open("wb") as log:
        try:
            result = subprocess.run(
                command,
                cwd=ROOT,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=60,
            )
        except subprocess.TimeoutExpired:
            stage["exit_code"] = 124
            stage["reason"] = "preparation timeout"
            report_path.write_text(json.dumps(report, indent=2) + "\n")
            raise
    stage["exit_code"] = result.returncode
    stage["log_sha256"] = hashlib.sha256(log_path.read_bytes()).hexdigest()
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"SOURCE-LIBRARY-DIAGNOSTIC: {log_path}: exit {result.returncode}", flush=True
    )
    result.check_returncode()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain-dir", type=Path, required=True)
    parser.add_argument("--rustup-home", type=Path)
    parser.add_argument(
        "--receipt", type=Path, default=Path("/tmp/mrr-source-proof.json")
    )
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--probe-worklists", action="store_true")
    parser.add_argument(
        "--probe-btree-source",
        action="store_true",
        help="Retain a bounded diagnostic extracting actual unsafe BTree source",
    )
    args = parser.parse_args()
    if args.probe_btree_source:
        args.probe_worklists = True
    if args.probe_worklists and args.update:
        parser.error("a native-adapter probe cannot replace proved generated modules")
    toolchain = args.toolchain_dir.resolve()
    environment = dict(os.environ)
    environment.pop("MRR_BTREE_SOURCE_PROBE_DIR", None)
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
            "mrr_agentic_ai_context::worklist::pop_identity",
            "mrr_agentic_ai_context::worklist::append_identities",
            "mrr_agentic_ai_context::worklist::initial_pending",
            "mrr_agentic_ai_context::evidence::admit_fact_evidence",
            "mrr_agentic_ai_context::state::expand_identity",
            "mrr_agentic_ai_context::state::advance_closure",
            "mrr_agentic_ai_context::state::run_closure",
            "mrr_agentic_ai_context::state::advance_impact",
            "mrr_agentic_ai_context::state::run_impact",
            "mrr_agentic_ai_context::state::build_reverse_index",
        ]
    )
    context = tempfile.TemporaryDirectory(prefix="mrr-source-proof-")
    if args.probe_btree_source:
        diagnostic = Path(
            tempfile.mkdtemp(prefix="mrr-btree-source-probe-", dir=receipt.parent)
        )
        environment["MRR_BTREE_SOURCE_PROBE_DIR"] = str(diagnostic)
        (diagnostic / "library-source-probe.json").write_text(
            json.dumps(
                {
                    "schema": "mrr.context.library-source-probe.v1",
                    "proven": False,
                    "aeneas": version.strip(),
                    "charon": charon_version.strip(),
                    "included_source": "alloc::collections::btree",
                    "stages": [],
                },
                indent=2,
            )
            + "\n"
        )
        context.cleanup()
        context = nullcontext(str(diagnostic))
        print(f"SOURCE-LIBRARY-PROBE: {diagnostic}", flush=True)
    with context as directory:
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
                "--include",
                "core::borrow",
                *(
                    ["--include", "alloc::collections::btree"]
                    if args.probe_btree_source
                    else []
                ),
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
        if args.probe_btree_source:
            raise RuntimeError(
                "library source translation is diagnostic only; no implementation proof"
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
        expected = {
            "Types.lean",
            "Funs.lean",
            "TypesExternal_Template.lean",
            "FunsExternal_Template.lean",
        }
        if {path.name for path in emitted} != expected:
            raise RuntimeError(
                "unexpected extraction surface or external-definition obligations"
            )
        allowed = {
            "TypesExternal_Template.lean": [
                "alloc.collections.btree.map.entry.OccupiedEntry",
                "alloc.collections.btree.map.entry.VacantEntry",
                "alloc.collections.btree.map.BTreeMap",
                "alloc.collections.btree.map.Iter",
                "alloc.collections.btree.set.BTreeSet",
                "alloc.collections.btree.set.Iter",
            ],
            "FunsExternal_Template.lean": [
                "Array.Insts.CoreCmpOrd.cmp",
                "alloc.collections.btree.map.entry.Entry.or_default",
                "alloc.collections.btree.map.BTreeMapKVGlobal.new",
                "alloc.collections.btree.map.BTreeMap.get",
                "alloc.collections.btree.map.BTreeMap.entry",
                "SharedABTreeMap.Insts.CoreIterTraitsCollectIntoIteratorPairSharedAKSharedAVIter.into_iter",
                "alloc.collections.btree.map.Iter.Insts.CoreIterTraitsIteratorIteratorPairSharedAKSharedAV.next",
                "alloc.collections.btree.set.BTreeSet.insert",
                "SharedABTreeSet.Insts.CoreIterTraitsCollectIntoIteratorSharedATIter.into_iter",
                "alloc.collections.btree.set.BTreeSetTGlobal.Insts.CoreDefaultDefault.default",
                "alloc.collections.btree.set.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.next",
            ],
        }
        for filename, declarations in allowed.items():
            actual = re.findall(
                r"^axiom\s+([^\s({:]+)",
                (output / "Generated" / filename).read_text(),
                re.MULTILINE,
            )
            if actual != declarations:
                raise RuntimeError(f"unmodeled library interface: {filename}: {actual}")
        hashes = {}
        for path in emitted:
            if "External_Template" in path.name:
                continue
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
                    "library_model_sha256": {
                        name: hashlib.sha256(
                            (PROJECT / "Generated" / name).read_bytes()
                        ).hexdigest()
                        for name in ("TypesExternal.lean", "FunsExternal.lean")
                    },
                    "library_model_interfaces": allowed,
                    "library_model_boundary": "trusted extensional BTree membership/lookup and entry restoration, map/set enumeration and lexicographic array ordering; no unsafe stdlib or sorted iteration proof",
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
