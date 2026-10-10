"""Orchestrate actual POO order schemas and Lean; no search implementation."""

import argparse
import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

from .process_runner import run


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--executions", type=Path, required=True)
    parser.add_argument("--orders", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    project = root / "proofs/MRRProof/SearchComposition"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    executions, orders = args.executions.resolve(), args.orders.resolve()
    entries = json.loads(orders.read_text())
    schema = json.loads(args.schema.read_text())
    Draft202012Validator.check_schema(schema)
    for entry in entries:
        Draft202012Validator(schema).validate(entry)
    if not entries:
        raise SystemExit("empty actual order witness inventory")
    print(f"SEARCH-ORDER-SCHEMA-OK: {len(entries)} witnesses", flush=True)

    def check(path: Path, name: str, rejection: str | None = None) -> dict:
        result = run(["lake", "env", "lean", "--run", "OrderChecks.lean", str(executions), str(path)],
                     cwd=project, log=output / f"{name}.log", label="SEARCH-ORDER")
        text = result.output.decode()
        if rejection is None:
            if result.status != 0 or text.count("SEARCH-ORDER-LEAN-OK:") != len(entries):
                raise SystemExit(f"actual source certificate check failed: {name}")
        elif result.status == 0 or rejection not in text:
            raise SystemExit(f"expected source certificate rejection missing: {name}")
        print(f"SEARCH-ORDER-CONTROL-OK: {name} exit={result.status}", flush=True)
        return {"name": name, "exit": result.status, "expectedRejection": rejection}

    checks = [check(orders, "actual-orders")]
    controls = [
        ("stale-generation", "exact execution receipt binding"),
        ("foreign-dag", "exact execution receipt binding"),
        ("missing-root", "role root factor inventory"),
        ("foreign-parent", "unique complete role graph"),
        ("role-cycle", "C3 source graph certification"),
        ("reversed-runtime-order", "runtime C4 agrees with both source certificates"),
        ("wrong-runtime-algorithm", "actual runtime algorithm"),
        ("omitted-evidence", "order schema and retained evidence"),
    ]
    for name, rejection in controls:
        mutated = copy.deepcopy(entries)
        entry = mutated[-1]
        if name == "stale-generation":
            entry["compositionReceipt"]["generationIdentity"] = "stale"
        elif name == "foreign-dag":
            entry["compositionReceipt"]["pooDagDigest"] = "foreign-dag"
        elif name == "missing-root":
            entry["roleRoots"].pop()
        elif name == "foreign-parent":
            entry["roleGraph"][0][1] = ["foreign-parent"]
        elif name == "role-cycle":
            entry["roleGraph"][0][1] = [entry["roleGraph"][0][0]]
        elif name == "reversed-runtime-order":
            entry["runtimeOrders"][0][1].reverse()
        elif name == "wrong-runtime-algorithm":
            entry["runtimeAlgorithm"] = "invented-order"
        elif name == "omitted-evidence":
            entry["complete"] = False
        path = output / f"{name}.v1.json"
        path.write_text(json.dumps(mutated, indent=2) + "\n")
        checks.append(check(path, name, rejection))
    paths = [executions, orders, args.schema.resolve(), project / "PlanOrders.lean",
             project / "OrderChecks.lean", Path(__file__).resolve()]
    receipt = {
        "schemaId": "mrr.search.plan-orders.qualification", "schemaVersion": "1",
        "checks": checks,
        "sources": [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                    for path in paths],
        "scope": "Actual emitted POO parent graphs and C4 orders checked by Lean C3/C4; no source refinement or scheduling claim",
    }
    (output / "order-qualification.v1.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
