"""Schema checks and negative-fixture orchestration; Lean owns admission."""

import argparse
import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

from .process_runner import run


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--witnesses", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    project = root / "proofs/MRRProof/SearchComposition"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    witnesses = args.witnesses.resolve()
    entries = json.loads(witnesses.read_text())
    schema = json.loads(args.schema.read_text())
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    if not entries:
        raise SystemExit("empty execution witness inventory")
    for entry in entries:
        validator.validate(entry)
    print(f"SEARCH-EXECUTION-SCHEMA-OK: {len(entries)} witnesses", flush=True)

    def check(path: Path, name: str, expected: str | None = None) -> dict:
        result = run(["lake", "env", "lean", "--run", "ExecutionChecks.lean", str(path)],
                     cwd=project, log=output / f"{name}.log", label="SEARCH-EXECUTION")
        text = result.output.decode()
        if expected is None:
            if result.status != 0 or text.count("SEARCH-EXECUTION-LEAN-OK:") != len(entries):
                raise SystemExit(f"positive execution check failed: {name}")
        elif result.status == 0 or expected not in text:
            raise SystemExit(f"negative execution check lacked expected rejection: {name}")
        print(f"SEARCH-EXECUTION-CONTROL-OK: {name} exit={result.status}", flush=True)
        return {"name": name, "exit": result.status, "expectedRejection": expected}

    checks = [check(witnesses, "execution-lean")]
    controls = [
        ("stale-generation", "branch binding"),
        ("wrong-owner", "Data candidate composition matches Lean semantics"),
        ("missing-path", "complete POO path inventory"),
        ("missing-influences", "complete MRR influence inventory"),
        ("incomplete-witness", "witness schema"),
        ("foreign-parent", "missing causal parent"),
    ]
    for name, rejection in controls:
        entry = copy.deepcopy(entries[-1])
        witness = entry["witness"]
        if name == "stale-generation":
            witness["branches"][0]["generation"] = "stale"
        elif name == "wrong-owner":
            entry["compositionReceipt"]["mergedOwnerIds"][0] = "foreign-owner"
        elif name == "missing-path":
            witness["paths"].pop()
        elif name == "missing-influences":
            witness["influences"] = []
        elif name == "incomplete-witness":
            entry["complete"] = False
            entry["witness"] = None
        elif name == "foreign-parent":
            witness["observations"][-1]["parents"] = ["foreign-parent"]
        path = output / f"{name}.v1.json"
        path.write_text(json.dumps([entry], indent=2) + "\n")
        checks.append(check(path, name, rejection))

    sources = [witnesses, args.schema.resolve(), project / "ExecutionChecks.lean",
               project / "ExecutionModel.lean", Path(__file__).resolve()]
    receipt = {
        "schemaId": "mrr.search.execution.qualification", "schemaVersion": "1",
        "modes": [entry["compositionReceipt"]["mode"] for entry in entries],
        "checks": checks,
        "sources": [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                    for path in sources],
        "scope": "Local one/two-leaf differential checking; no Rust/Scheme source refinement claim",
    }
    (output / "execution-qualification.v1.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
