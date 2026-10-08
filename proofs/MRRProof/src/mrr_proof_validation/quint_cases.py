"""Pinned finite inputs and Quint trace projections shared by proof gates."""

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


def revision_coverage_instance(source: Path, states: list[dict], directory: Path) -> Path:
    """Add a pure invariant to the exact instance used for the first check."""
    if not states:
        raise ValueError("empty Quint replay state set")
    entries = []
    for state in states:
        if state.get("expected") != "accept":
            raise ValueError("counterexample in positive Quint replay state set")
        entries.append("(" + ", ".join([
            json.dumps(state["phase"]),
            quint_set(state["todo"]),
            quint_set(state["changed"]),
            quint_set(state["invalidated"]),
            quint_set(state["reusable"]),
        ]) + ")")
    original = source.read_text()
    if not original.endswith("}\n"):
        raise ValueError("unexpected Quint instance ending")
    path = directory / source.name
    path.write_text(
        original[:-2]
        + "  val replayedStates = Set(\n    "
        + ",\n    ".join(entries)
        + "\n  )\n"
        + "  val allStatesReplayed = replayedStates.contains("
        + "(phase, todo, changed, invalidated, reusable))\n"
        + "}\n"
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


def _integer_set(value: dict) -> list[int]:
    if set(value) != {"#set"}:
        raise ValueError("Quint trace state field is not an ITF set")
    numbers = []
    for item in value["#set"]:
        if set(item) != {"#bigint"}:
            raise ValueError("Quint ITF set contains a non-integer")
        numbers.append(int(item["#bigint"]))
    if len(numbers) != len(set(numbers)):
        raise ValueError("duplicate Quint set member")
    return sorted(numbers)


def revision_itf_states(trace: Path, scenario: str) -> list[dict]:
    document = json.loads(trace.read_text())
    if document.get("#meta", {}).get("status") != "ok":
        raise ValueError("Quint trace did not finish normally")
    case = revision_inputs(scenario)
    result = []
    for raw in document["states"]:
        fields = {key.split("::")[-1]: value for key, value in raw.items()
                  if key != "#meta"}
        if set(fields) != {"phase", "todo", "changed", "invalidated", "reusable"}:
            raise ValueError("Quint trace state fields differ from the Lean projection")
        result.append({
            "expected": "accept",
            "phase": fields["phase"],
            "todo": _integer_set(fields["todo"]),
            "changed": _integer_set(fields["changed"]),
            "invalidated": _integer_set(fields["invalidated"]),
            "reusable": _integer_set(fields["reusable"]),
            "source": [0, 1, 2, 3],
            "oldEdges": case["oldEdges"],
            "newEdges": case["newEdges"],
            "global": case["global"],
            "unequal": case["unequal"],
            "oldSelected": case["oldSelected"],
            "newSelected": case["newSelected"],
        })
    return result


def revision_counterexample(output: str, scenario: str) -> dict:
    matches = list(re.finditer(r"(?m)^State (\d+):", output))
    if not matches or [int(match[1]) for match in matches] != list(range(1, len(matches) + 1)):
        raise ValueError("missing or nonconsecutive Quint counterexample states")
    body = output[matches[-1].end():]
    fields = dict(re.findall(
        r"(?m)^/\\ RevisionCase_RevisionLifecycle_(\w+) = (.+)$", body
    ))
    if not {"phase", "todo", "changed", "invalidated", "reusable"} <= set(fields):
        raise ValueError("Quint counterexample state fields differ from Lean projection")

    def integers(name: str) -> list[int]:
        value = fields[name].strip()
        if not re.fullmatch(r"\{(?:\d+(?:,\s*\d+)*)?\}", value):
            raise ValueError(f"invalid Quint counterexample set: {name}")
        return sorted(int(number) for number in re.findall(r"\d+", value))

    case = revision_inputs(scenario)
    return {
        "expected": "reject",
        "phase": json.loads(fields["phase"]),
        "todo": integers("todo"),
        "changed": integers("changed"),
        "invalidated": integers("invalidated"),
        "reusable": integers("reusable"),
        "source": [0, 1, 2, 3],
        "oldEdges": case["oldEdges"],
        "newEdges": case["newEdges"],
        "global": case["global"],
        "unequal": case["unequal"],
        "oldSelected": case["oldSelected"],
        "newSelected": case["newSelected"],
    }
