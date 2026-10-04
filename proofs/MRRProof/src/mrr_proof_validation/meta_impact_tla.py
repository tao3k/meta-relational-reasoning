#!/usr/bin/env python3
"""Qualify finite Meta Impact scenarios and required TLC counterexamples."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import tempfile

from . import native_tests

ROOT = Path(__file__).resolve().parents[4]
MODEL = ROOT / "proofs/MRRProof/AgenticAIContextTLA"
TLC_HASH = "936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"


class Output:
    def __init__(self, log: Path):
        self.log = log
        self.data = bytearray()

    def observe(self, chunk: bytes) -> None:
        self.data.extend(chunk)
        with self.log.open("ab") as stream:
            stream.write(chunk)

    def valid(self) -> bool:
        return (
            b"TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)" in self.data
            and b"Model checking completed. No error has been found." in self.data
            and b"0 states left on queue" in self.data
            and b"Finished in" in self.data
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--tlc-jar", type=Path)
    args = parser.parse_args()
    args.receipt = args.receipt.resolve()
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    command = ["tlc"]
    jar_hash = None
    if args.tlc_jar:
        jar_hash = hashlib.sha256(args.tlc_jar.read_bytes()).hexdigest()
        if jar_hash != TLC_HASH:
            parser.error("TLC jar differs from pinned official v1.7.4")
        command = ["java", "-XX:+UseParallelGC", "-cp", str(args.tlc_jar.resolve()), "tlc2.TLC"]

    cases = [(name, "none", None) for name in
             ("alternative", "last", "incomplete", "absence", "newEdge", "temporal")]
    cases += [
        ("alternative", "dropAlternative", "NoFalseRetraction"),
        ("newEdge", "dropNewEdges", "FrontierSound"),
        ("absence", "uncertifiedAbsence", "NoUncertifiedAbsence"),
        ("temporal", "promoteCausal", "NoTemporalPromotion"),
    ]
    template = (MODEL / "MetaImpactCases.cfg").read_text()
    results = []
    with tempfile.TemporaryDirectory(prefix="mrr-meta-impact-", dir=args.receipt.parent) as directory:
        for model in MODEL.glob("*.tla"):
            (Path(directory) / model.name).write_bytes(model.read_bytes())
        for scenario, bug, invariant in cases:
            name = f"{scenario}-{bug}"
            config = template.replace('Scenario = "alternative"', f'Scenario = "{scenario}"')
            config = config.replace('Bug = "none"', f'Bug = "{bug}"')
            config_path = Path(directory) / f"{name}.cfg"
            config_path.write_text(config)
            log = args.receipt.parent / f"meta-impact-{name}.log"
            log.unlink(missing_ok=True)
            output = Output(log)
            print(f"META-IMPACT-TLA: {name}", flush=True)
            status = native_tests.qualify(
                [*command, "-workers", "1", "-seed", "1", "-fp", "0",
                 "-metadir", str(Path(directory) / name), "-config", config_path.name,
                 "MetaImpactCases"],
                output,
                cwd=directory,
            )
            content = output.data.decode(errors="replace")
            states = re.search(r"([\d,]+) states generated, ([\d,]+) distinct states", content)
            if states is None:
                print(f"META-IMPACT-TLA-FAILED: no state count for {name}", flush=True)
                return 1
            if invariant is None:
                accepted = (
                    status == 0
                    and "Model checking completed. No error has been found." in content
                    and "0 states left on queue" in content
                )
            else:
                accepted = status == 12 and f"Invariant {invariant} is violated." in content
            if not accepted:
                print(f"META-IMPACT-TLA-FAILED: {name} exit {status}", flush=True)
                return 1
            results.append({
                "scenario": scenario,
                "mutation": bug,
                "expected_invariant": invariant,
                "exit": status,
                "generated": int(states[1].replace(",", "")),
                "distinct": int(states[2].replace(",", "")),
                "config_sha256": hashlib.sha256(config.encode()).hexdigest(),
                "log": str(log),
                "log_sha256": hashlib.sha256(output.data).hexdigest(),
            })

    args.receipt.write_text(json.dumps({
        "schema": "mrr.context.meta-impact-tla.v1",
        "scope": "finite support, absence, frontier, and Temporal receipt scenarios",
        "tlc": {"version": "2.19", "revision": "5a47802", "jar_sha256": jar_hash},
        "models": {name: hashlib.sha256((MODEL / name).read_bytes()).hexdigest()
                   for name in ("MetaImpact.tla", "MetaImpactCases.tla")},
        "cases": results,
    }, indent=2) + "\n")
    print(f"META-IMPACT-TLA-OK: {len(results)} cases", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
