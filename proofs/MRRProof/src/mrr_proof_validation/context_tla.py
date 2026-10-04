#!/usr/bin/env python3
"""Check finite revision lifecycles and require real mutation counterexamples."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import tempfile

from . import native_tests

ROOT = Path(__file__).resolve().parents[4]
MODEL = ROOT / "proofs/MRRProof/AgenticAIContextTLA"


class TLCReceipt:
    def __init__(self, log: Path):
        self.log = log
        self.output = bytearray()

    def observe(self, chunk: bytes) -> None:
        self.output.extend(chunk)
        with self.log.open("ab") as stream:
            stream.write(chunk)

    def valid(self) -> bool:
        return (
            b"TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)" in self.output
            and b"Model checking completed. No error has been found." in self.output
            and b"0 states left on queue" in self.output
            and b"Finished in" in self.output
        )


def exported_states(text: str, expected: str) -> list[dict]:
    states = []
    for block in re.findall(r"(?m)^State [0-9]+:[^\n]*\n((?:/\\[^\n]*\n)+)", text):
        fields = dict(re.findall(r"(?m)^/\\ (\w+) = (.*)$", block))
        required = {
            "phase",
            "todo",
            "changed",
            "invalidated",
            "reusable",
            "source",
            "oldEdges",
            "newEdges",
            "global",
            "unequal",
            "oldSelected",
            "newSelected",
        }
        if set(fields) != required:
            raise ValueError("TLC state fields differ from the shared projection")
        state = {"expected": expected}
        for key, value in fields.items():
            if key == "phase":
                state[key] = json.loads(value)
            elif key == "global":
                if value not in ("TRUE", "FALSE"):
                    raise ValueError("invalid TLC Boolean")
                state[key] = value == "TRUE"
            elif key in ("oldEdges", "newEdges"):
                if not re.fullmatch(
                    r"\{(?:<<[0-9]+, [0-9]+>>(?:, <<[0-9]+, [0-9]+>>)*)?\}", value
                ):
                    raise ValueError("invalid TLC edge set")
                state[key] = [
                    [int(a), int(b)]
                    for a, b in re.findall(r"<<([0-9]+), ([0-9]+)>>", value)
                ]
            else:
                if not re.fullmatch(r"\{(?:[0-9]+(?:, [0-9]+)*)?\}", value):
                    raise ValueError("invalid TLC identity set")
                state[key] = [int(number) for number in re.findall(r"[0-9]+", value)]
        states.append(state)
    # TLC can dump a state more than once while building/checking the temporal
    # graph. Preserve the first appearance, then require exact distinct coverage.
    return list({json.dumps(state, sort_keys=True): state for state in states}.values())


class LeanReceipt(TLCReceipt):
    def __init__(self, log: Path, count: int):
        super().__init__(log)
        self.count = count

    def valid(self) -> bool:
        marker = f"CONTEXT-TLA-LEAN-OK: {self.count} actual TLC states".encode()
        return (
            marker in self.output
            and self.output.count(b"PASS: TLA/Lean state ") == self.count
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--tlc-jar", type=Path)
    parser.add_argument("--lean-replay", type=Path, required=True)
    args = parser.parse_args()
    args.receipt = args.receipt.resolve()
    command = ["tlc"]
    jar_hash = None
    if args.tlc_jar:
        jar_hash = hashlib.sha256(args.tlc_jar.read_bytes()).hexdigest()
        if (
            jar_hash
            != "936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
        ):
            parser.error("TLC jar differs from pinned official v1.7.4")
        command = [
            "java",
            "-XX:+UseParallelGC",
            "-cp",
            str(args.tlc_jar.resolve()),
            "tlc2.TLC",
        ]
    args.receipt.unlink(missing_ok=True)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    cases = [
        (name, "none")
        for name in (
            "cycle",
            "removed",
            "added",
            "diamond",
            "self",
            "global",
            "equal",
            "selection",
        )
    ] + [("added", "oldEdgesOnly"), ("removed", "publishEarly")]
    template = (MODEL / "RevisionCases.cfg").read_text()
    results = []
    all_states = []
    with tempfile.TemporaryDirectory(
        prefix="mrr-tlc-", dir=args.receipt.parent
    ) as directory:
        for model in MODEL.glob("*.tla"):
            (Path(directory) / model.name).write_bytes(model.read_bytes())
        for scenario, bug in cases:
            name = f"{scenario}-{bug}"
            config = template.replace('Scenario = "cycle"', f'Scenario = "{scenario}"')
            config = config.replace('Bug = "none"', f'Bug = "{bug}"')
            config_path = Path(directory) / f"{name}.cfg"
            config_path.write_text(config)
            log_path = args.receipt.parent / f"context-tla-{name}.log"
            log_path.unlink(missing_ok=True)
            observed = TLCReceipt(log_path)
            print(f"CONTEXT-TLA-QUALIFY: {name}", flush=True)
            status = native_tests.qualify(
                [
                    *command,
                    "-workers",
                    "1",
                    "-seed",
                    "1",
                    "-fp",
                    "0",
                    "-dump",
                    str(Path(directory) / f"{name}.states"),
                    "-metadir",
                    str(Path(directory) / name),
                    "-config",
                    config_path.name,
                    "RevisionCases",
                ],
                observed,
                cwd=directory,
            )
            output = observed.output.decode(errors="replace")
            counterexample = "Invariant CompleteBeforePublish is violated." in output
            accepted = (
                status == 0 and observed.valid()
                if bug == "none"
                else status == 12 and counterexample
            )
            if not accepted:
                print(f"CONTEXT-TLA-FAILED: {name} exit {status}", flush=True)
                return 1
            if bug == "none":
                dump = Path(directory) / f"{name}.states.dump"
                projected = exported_states(dump.read_text(), "accept")
                if not any(state["phase"] == "published" for state in projected):
                    raise ValueError("no published TLC state")
            else:
                projected = exported_states(output, "reject")[-1:]
                if not projected:
                    raise ValueError("missing actual TLC counterexample state")
            all_states.extend(projected)
            states = re.search(
                r"([\d,]+) states generated, ([\d,]+) distinct states", output
            )
            if states is None:
                return 1
            if bug == "none" and len(projected) != int(states[2].replace(",", "")):
                raise ValueError(
                    "Lean projection did not include every distinct TLC state"
                )
            results.append(
                {
                    "scenario": scenario,
                    "mutation": bug,
                    "exit": status,
                    "counterexample": counterexample,
                    "generated": int(states[1].replace(",", "")),
                    "distinct": int(states[2].replace(",", "")),
                    "config_sha256": hashlib.sha256(config.encode()).hexdigest(),
                    "log": str(log_path),
                    "log_sha256": hashlib.sha256(observed.output).hexdigest(),
                }
            )
    state_path = args.receipt.parent / "context-tla-states.json"
    state_path.write_text(json.dumps(all_states, indent=2) + "\n")
    replay_log = args.receipt.parent / "context-tla-lean.log"
    replay_log.unlink(missing_ok=True)
    lean = LeanReceipt(replay_log, len(all_states))
    if (
        native_tests.qualify(
            [str(args.lean_replay.resolve()), str(state_path.resolve())], lean
        )
        != 0
    ):
        return 1
    args.receipt.write_text(
        json.dumps(
            {
                "schema": "mrr.context.tla-revision.v1",
                "tlc": {
                    "version": "2.19",
                    "revision": "5a47802",
                    "jar_sha256": jar_hash,
                },
                "lean_replay": {
                    "states": len(all_states),
                    "binary_sha256": hashlib.sha256(
                        args.lean_replay.read_bytes()
                    ).hexdigest(),
                    "states_sha256": hashlib.sha256(
                        state_path.read_bytes()
                    ).hexdigest(),
                    "log_sha256": hashlib.sha256(lean.output).hexdigest(),
                },
                "scope": "finite four-identity lifecycle scenarios; not a Rust source proof",
                "models": {
                    p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                    for p in sorted(MODEL.glob("*.tla"))
                },
                "cases": results,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"CONTEXT-TLA-OK: {len(results)} checked cases", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
