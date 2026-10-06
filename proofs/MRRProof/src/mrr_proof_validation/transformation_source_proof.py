"""Check fresh extraction of the production finite transport table operations."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

from .context_source_proof import AENEAS_VERSION, CHARON_REVISION, ROOT


FUNCTIONS = [
    f"meta_relational_reasoning::transformation_table::{name}"
    for name in ("forward", "solve", "extract")
]
PROJECT = ROOT / "proofs/MRRProof/AgenticAIContextRust"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain-dir", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    tools = args.toolchain_dir.resolve()
    environment = dict(os.environ)
    environment["PATH"] = f"{Path.home() / '.cargo/bin'}:{environment['PATH']}"
    environment["CARGO_TARGET_DIR"] = str(receipt.parent / "transformation-source-target")
    versions = {}
    for tool, argument, expected in (
        ("aeneas", "-version", AENEAS_VERSION),
        ("charon", "version", CHARON_REVISION),
    ):
        version = subprocess.check_output(
            [tools / tool, argument], env=environment, text=True, timeout=30
        ).strip()
        if expected not in version:
            raise RuntimeError(f"wrong pinned {tool}: {version}")
        versions[tool] = version
    environment["CHARON_MIRI_SYSROOTS"] = subprocess.check_output(
        [tools / "charon", "toolchain-path"], env=environment, text=True, timeout=30
    ).strip()

    def run(command: list[str]) -> None:
        print("TRANSFORMATION-SOURCE:", " ".join(map(str, command)), flush=True)
        subprocess.run(command, cwd=ROOT, env=environment, check=True, timeout=300)

    with tempfile.TemporaryDirectory(prefix="mrr-transformation-source-") as directory:
        output = Path(directory)
        llbc = output / "transformation.llbc"
        run([
            str(tools / "charon"), "cargo", "--preset=aeneas", "--sysroot", "default",
            *[arg for name in FUNCTIONS for arg in ("--start-from", name)],
            "--dest-file", str(llbc), "--", "--package", "meta-relational-reasoning",
            "--lib", "--no-default-features", "--locked", "--offline",
        ])
        run([
            str(tools / "aeneas"), "-backend", "lean", "-dest", str(output),
            "-subdir", "TransformGenerated", "-namespace", "MRR.TransformationRust",
            "-split-files", str(llbc),
        ])
        emitted = sorted((output / "TransformGenerated").glob("*.lean"))
        if {path.name for path in emitted} != {"Types.lean", "Funs.lean"}:
            raise RuntimeError("unexpected extraction or external obligations")
        hashes = {}
        for path in emitted:
            source = path.read_text()
            if re.search(r"\b(?:axiom|sorry|admit)\b", source):
                raise RuntimeError(f"unproved generated declaration: {path.name}")
            content = source.translate(str.maketrans({"\u2190": "<-", "\u2192": "->"})).encode("ascii")
            target = PROJECT / "TransformGenerated" / path.name
            target.parent.mkdir(parents=True, exist_ok=True)
            if args.update:
                target.write_bytes(content)
            elif target.read_bytes() != content:
                raise RuntimeError(f"stale production extraction: {target}")
            hashes[path.name] = hashlib.sha256(content).hexdigest()
        receipt.write_text(json.dumps({
            "schema": "mrr.transformation.source-extraction.v1",
            **versions, "functions": FUNCTIONS,
            "llbc_sha256": hashlib.sha256(llbc.read_bytes()).hexdigest(),
            "generated_sha256": hashes,
            "source_sha256": {
                name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
                for name in (
                    "crates/meta-relational-reasoning/src/transformation_table.rs",
                    "crates/meta-relational-reasoning/src/transformation_finite.rs",
                )
            },
            "boundary": "production table access only; standard Slice/Vec models trusted; catalog transport laws, authority, codecs and dispatch are separate obligations",
        }, indent=2) + "\n")
    print("TRANSFORMATION-SOURCE-EXTRACTION-OK", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
