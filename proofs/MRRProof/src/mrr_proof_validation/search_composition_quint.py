"""Exhaust finite search publication states and require mutation counterexamples."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from . import process_runner, quint_cases


def snapshot_literal(state: dict) -> str:
    """Encode an already checked Lean snapshot as a Quint state tuple."""
    def binding(generation: int) -> str:
        return "{ scope: 0, source: 0, resident: 0, abi: 0, generation: " + str(generation) + " }"
    fields = [json.dumps(state["phase"]), binding(state["current"]), binding(state["observed"])]
    fields += [str(state[name]).lower() for name in (
        "complete", "truncated", "secondaryComplete", "secondaryTruncated", "inferred")]
    fields.append("Set(" + ", ".join(map(str, state["output"])) + ")")
    return "(" + ", ".join(fields) + ")"


def coverage_instance(mode: str, scenario: str, states: list[dict]) -> str:
    return (
        'module Coverage {\n'
        f'  import SearchComposition(BUG="{scenario}", MODE="{mode}").* from "./SearchComposition"\n'
        '  val replayedStates = Set(' + ", ".join(map(snapshot_literal, states)) + ')\n'
        '  val allStatesReplayed = and { safety, replayedStates.contains(\n'
        '    (phase, current, observed, complete, truncated, secondaryComplete, secondaryTruncated, inferred, output)) }\n'
        '}\n'
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    receipt = args.receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.unlink(missing_ok=True)
    executable = os.environ.get("MRR_QUINT_BIN") or shutil.which("quint")
    if not executable:
        parser.error("Quint executable is required")
    quint = str(Path(executable).resolve())
    version = subprocess.check_output([quint, "--version"], text=True, timeout=5).strip()
    if version != "0.33.0":
        parser.error(f"expected Quint 0.33.0, found {version!r}")
    cases = [(mode, "none") for mode in ("single", "intersect", "rankJoin")]
    cases += [("single", "partialSingle"), ("intersect", "disjointTruth")]
    positive_cases = set(cases)
    cases += [("intersect", bug) for bug in (
        "ignoreGeneration", "sourceSwap", "scopeSwap", "residentSwap", "abiSwap",
        "incompleteTruth", "incompleteSecondary", "skipInference", "emptyIntersection")]
    cases += [("rankJoin", "filterPrimary")]
    project = quint_cases.ROOT / "proofs/MRRProof/SearchComposition"
    replay_path = receipt.parent / "lean-replay.v1.json"
    replay_path.unlink(missing_ok=True)
    replay_log = receipt.parent / "lean-replay.log"
    replay_result = process_runner.run(
        ["lake", "env", "lean", "--run", "Replay.lean", str(replay_path)],
        cwd=project, log=replay_log, label="SEARCH-LEAN-REPLAY")
    if replay_result.status != 0:
        raise SystemExit(f"SEARCH-LEAN-REPLAY-FAILED: log={replay_log}")
    replay = json.loads(replay_path.read_text())
    if replay.get("schema") != "mrr.search.replay.v1":
        raise SystemExit("SEARCH-LEAN-REPLAY-FAILED: unknown schema")
    replay_cases = {(row["mode"], row["scenario"]): row["states"] for row in replay["cases"]}
    if set(replay_cases) != positive_cases or len(replay["cases"]) != len(positive_cases):
        raise SystemExit("SEARCH-LEAN-REPLAY-FAILED: missing or duplicate case")
    rows = []
    omission_control = None
    with tempfile.TemporaryDirectory(prefix="search-quint-", dir=receipt.parent) as directory:
        work = Path(directory)
        model = quint_cases.MODELS / "SearchComposition.qnt"
        shutil.copyfile(model, work / model.name)
        config = work / "tlc-config.json"
        config.write_text('{"workers":"1","maxHeap":"-Xmx1G"}\n')
        for mode, bug in cases:
            name = f"{mode}-{bug}"
            instance = work / "Case.qnt"
            instance.write_text(
                'module Case {\n'
                f'  import SearchComposition(BUG="{bug}", MODE="{mode}").* '
                'from "./SearchComposition"\n}\n')
            print(f"SEARCH-QUINT-QUALIFY: {name}", flush=True)
            log = receipt.parent / f"search-quint-{name}.log"
            result = process_runner.run([
                quint, "verify", instance.name, "--main", "Case", "--backend", "tlc",
                "--apalache-version", "0.62.1", "--server-endpoint", "127.0.0.1:8866",
                "--invariant", "safety", "--tlc-config", str(config), "--verbosity", "3",
            ], cwd=work, log=log)
            output = result.output.decode(errors="replace")
            try:
                generated, distinct, remaining = quint_cases.checker_counts(output)
            except ValueError as error:
                raise SystemExit(f"SEARCH-QUINT-FAILED: {name}: {error}; log={log}") from error
            if (mode, bug) in positive_cases:
                accepted = result.status == 0 and "[ok] No violation found" in output and remaining == 0
            else:
                accepted = (result.status == 1 and "Error: Invariant q_inv is violated." in output
                            and "error: found a counterexample" in output)
            if not accepted:
                raise SystemExit(f"SEARCH-QUINT-FAILED: {name}; log={log}")
            rows.append({"mode": mode, "mutation": bug, "exit": result.status,
                         "generated": generated, "distinct": distinct, "remaining": remaining,
                         "instance_sha256": quint_cases.sha256(instance.read_bytes()),
                         "log": str(log), "log_sha256": quint_cases.sha256(result.output)})
            print(f"SEARCH-QUINT-OK: {name} distinct={distinct}", flush=True)
            if (mode, bug) in positive_cases:
                states = replay_cases[(mode, bug)]
                if len(states) != distinct or len(set(map(snapshot_literal, states))) != len(states):
                    raise SystemExit(f"SEARCH-REPLAY-FAILED: {name}: state cardinality mismatch")
                coverage = work / "Coverage.qnt"
                coverage.write_text(coverage_instance(mode, bug, states))
                coverage_log = receipt.parent / f"search-coverage-{name}.log"
                coverage_result = process_runner.run([
                    quint, "verify", coverage.name, "--main", "Coverage", "--backend", "tlc",
                    "--apalache-version", "0.62.1", "--server-endpoint", "127.0.0.1:8866",
                    "--invariant", "allStatesReplayed", "--tlc-config", str(config), "--verbosity", "3",
                ], cwd=work, log=coverage_log)
                coverage_output = coverage_result.output.decode(errors="replace")
                _, coverage_distinct, coverage_remaining = quint_cases.checker_counts(coverage_output)
                if not (coverage_result.status == 0 and "[ok] No violation found" in coverage_output
                        and coverage_remaining == 0 and coverage_distinct == distinct):
                    raise SystemExit(f"SEARCH-REPLAY-FAILED: {name}: log={coverage_log}")
                rows[-1]["replay"] = {
                    "states": len(states), "coverage_distinct": coverage_distinct,
                    "remaining": coverage_remaining, "exit": coverage_result.status,
                    "instance_sha256": quint_cases.sha256(coverage.read_bytes()),
                    "log": str(coverage_log), "log_sha256": quint_cases.sha256(coverage_result.output),
                }
                print(f"SEARCH-REPLAY-COVERAGE-OK: {name} states={len(states)}", flush=True)
                if omission_control is None:
                    omitted = next(state for state in states if state["phase"] == "published")
                    coverage.write_text(coverage_instance(mode, bug, [state for state in states if state != omitted]))
                    omission_log = receipt.parent / "search-replay-omission.log"
                    omission_result = process_runner.run([
                        quint, "verify", coverage.name, "--main", "Coverage", "--backend", "tlc",
                        "--apalache-version", "0.62.1", "--server-endpoint", "127.0.0.1:8866",
                        "--invariant", "allStatesReplayed", "--tlc-config", str(config), "--verbosity", "3",
                    ], cwd=work, log=omission_log)
                    omission_output = omission_result.output.decode(errors="replace")
                    if not (omission_result.status == 1 and "Error: Invariant q_inv is violated." in omission_output
                            and "error: found a counterexample" in omission_output):
                        raise SystemExit(f"SEARCH-REPLAY-OMISSION-FAILED: log={omission_log}")
                    omission_control = {
                        "mode": mode, "scenario": bug, "omitted": omitted, "exit": omission_result.status,
                        "instance_sha256": quint_cases.sha256(coverage.read_bytes()),
                        "log": str(omission_log), "log_sha256": quint_cases.sha256(omission_result.output),
                    }
                    print("SEARCH-REPLAY-OMISSION-OK: required counterexample found", flush=True)
    receipt.write_text(json.dumps({
        "schema": "mrr.search.composition.quint.v1",
        "scope": "finite search binding and publication safety; not Rust source refinement",
        "quint": version, "backend": "tlc", "apalache": "0.62.1",
        "model_sha256": quint_cases.sha256(model.read_bytes()), "cases": rows,
        "lean_replay": {"path": str(replay_path), "sha256": quint_cases.sha256(replay_path.read_bytes()),
                        "log": str(replay_log), "log_sha256": quint_cases.sha256(replay_result.output)},
        "lean_sources_sha256": {path.name: quint_cases.sha256(path.read_bytes()) for path in project.glob("*.lean")},
        "omission_control": omission_control,
    }, indent=2) + "\n")
    print(f"SEARCH-QUINT-OK: {len(rows)} checked cases", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
