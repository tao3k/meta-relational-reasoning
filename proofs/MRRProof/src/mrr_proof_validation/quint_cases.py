"""Pinned finite inputs and checker counts shared by Quint proof gates."""

import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[4]
MODELS = ROOT / "proofs/quint"
CASES_PATH = MODELS / "cases.json"
CASES = json.loads(CASES_PATH.read_text())
if CASES.get("schema") != "mrr.quint-cases.v1":
    raise ValueError("unknown Quint case schema")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def quint_set(values: list) -> str:
    entries = []
    for value in values:
        if isinstance(value, int):
            entries.append(str(value))
        elif (isinstance(value, list) and len(value) == 2
              and all(isinstance(part, int) for part in value)):
            entries.append(f"({value[0]}, {value[1]})")
        else:
            raise ValueError(f"invalid finite Quint set member: {value!r}")
    return f"Set({', '.join(entries)})"


def revision_inputs(scenario: str) -> dict:
    source = CASES["revision"][scenario]
    return {
        **source,
        "oldSelected": source.get("oldSelected", [0, 1, 2, 3]),
        "newSelected": source.get("newSelected", [0, 1, 2, 3]),
    }


def instance(model: str, scenario: str, bug: str, directory: Path) -> Path:
    if model == "RevisionLifecycle":
        case = revision_inputs(scenario)
        bindings = {
            "OLD_EDGES": quint_set(case["oldEdges"]),
            "NEW_EDGES": quint_set(case["newEdges"]),
            "UNEQUAL": quint_set(case["unequal"]),
            "GLOBAL": str(case["global"]).lower(),
            "OLD_SELECTED": quint_set(case["oldSelected"]),
            "NEW_SELECTED": quint_set(case["newSelected"]),
            "BUG": json.dumps(bug),
        }
        name = "RevisionCase"
    elif model == "MetaImpact":
        case = CASES["metaImpact"][scenario]
        bindings = {
            "OLD_SUPPORTS": quint_set(case["oldSupports"]),
            "NEW_SUPPORTS": quint_set(case["newSupports"]),
            "OLD_PRESENT": quint_set(case["oldPresent"]),
            "NEW_PRESENT": quint_set(case["newPresent"]),
            "SUPPORT_COMPLETE": str(case["supportComplete"]).lower(),
            "COVERAGE_COMPLETE": str(case["coverageComplete"]).lower(),
            "OWNER_CLAIM": json.dumps(case["ownerClaim"]),
            "BUG": json.dumps(bug),
        }
        name = "MetaImpactCase"
    else:
        raise ValueError(f"unknown Quint model: {model}")
    arguments = ", ".join(f"{key} = {value}" for key, value in bindings.items())
    path = directory / f"{name}.qnt"
    path.write_text(
        (MODELS / f"{model}.qnt").read_text()
        + f"\nmodule {name} {{\n  import {model}({arguments}).*\n}}\n"
    )
    return path


def checker_counts(output: str) -> tuple[int, int, int]:
    counts = re.findall(
        r"([\d,]+) states generated, ([\d,]+) distinct states found, "
        r"([\d,]+) states left on queue\.", output
    )
    if not counts:
        raise ValueError("Quint checker did not report finite state counts")
    return tuple(int(number.replace(",", "")) for number in counts[-1])
