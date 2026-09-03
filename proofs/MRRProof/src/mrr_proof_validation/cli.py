#!/usr/bin/env python3
"""Validate MRR proof impact and kernel-check it with pinned local Lean."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import tomllib
from collections import deque
from pathlib import Path

from .org_babel import validate_org_babel_contract

ROOT = Path(__file__).resolve().parents[4]
CORE_CRATES = (
    "gql-source",
    "gql-syntax",
    "gql-ast",
    "gql-types",
    "gql-catalog",
    "gql-ir",
    "gql-sema",
    "gql-compiler",
    "gql-core",
    "gql",
    "mrr-frontends",
    "mrr-identity",
    "mrr-intent",
    "mrr-relation",
    "mrr-revision",
    "mrr-query",
    "mrr-logic",
    "mrr-lineage",
    "mrr-transition",
    "mrr-bundle",
    "mrr-ascent",
    "meta-relational-reasoning",
)
PROOF_OBLIGATIONS = {
    "gql-syntax": {
        "graph_element_predicate_admission_is_typed",
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_admission_is_typed",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "gql-ast": {
        "graph_element_predicate_admission_is_typed",
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_admission_is_typed",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "gql-ir": {
        "graph_element_predicate_admission_is_typed",
        "graph_match_path_prefix_admission_is_typed",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "gql-sema": {
        "graph_element_predicate_admission_is_typed",
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_admission_is_typed",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "gql-compiler": {
        "graph_element_predicate_admission_is_typed",
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_admission_is_typed",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "gql": {
        "graph_element_predicate_admission_is_typed",
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_admission_is_typed",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "mrr-frontends": {
        "graph_element_predicate_rejection_emits_no_ir",
        "graph_match_path_prefix_rejection_emits_no_ir",
        "order_page_admission_is_typed",
        "order_page_rejection_emits_no_ir",
        "filter_for_admission_is_typed",
        "filter_for_rejection_emits_no_ir",
        "primitive_result_admission_is_typed",
        "primitive_result_rejection_emits_no_ir",
    },
    "mrr-query": {"order_page_admission_is_typed"},
    "mrr-identity": {"identity_domain_separation"},
    "mrr-intent": {"identity_domain_separation"},
    "mrr-revision": {"identity_domain_separation"},
    "mrr-ascent": {
        "admitted_fact_ids_are_unique",
        "inserted_fact_has_admitted_schema",
        "admitted_rule_uses_only_admitted_relations",
    },
    "mrr-relation": {"inserted_fact_has_admitted_schema"},
    "mrr-logic": {
        "admitted_rule_uses_only_admitted_relations",
        "admitted_derivation_has_rule_and_premises",
    },
    "mrr-lineage": {"admitted_derivation_has_rule_and_premises"},
    "mrr-transition": {
        "inserted_fact_has_admitted_schema",
        "admitted_fact_ids_are_unique",
        "returned_counterexample_is_valid",
    },
    "mrr-bundle": {
        "inserted_fact_has_admitted_schema",
        "admitted_rule_uses_only_admitted_relations",
        "admitted_fact_ids_are_unique",
    },
    "meta-relational-reasoning": {
        "admitted_closure_binding_is_exact",
        "closure_admission_rejects_any_failed_owner",
        "returned_counterexample_is_valid",
    },
}
COUNTEREXAMPLE_FIXTURE = (
    ROOT / "proofs/MRRProof/fixtures/counterexample-receipt.json"
)
LOCAL_LEAN_TOOLCHAIN = "leanprover/lean4:v4.31.0"
LOCAL_LEAN_VERSION = "version 4.31.0"


def load_toml(path: Path) -> dict:
    with path.open("rb") as source:
        return tomllib.load(source)


def dependency_graph() -> dict[str, set[str]]:
    workspace = load_toml(ROOT / "Cargo.toml")
    declared = workspace["workspace"]["dependencies"]
    graph: dict[str, set[str]] = {}
    for crate in CORE_CRATES:
        manifest = load_toml(ROOT / "crates" / crate / "Cargo.toml")
        dependencies = manifest.get("dependencies", {})
        internal = {name for name in dependencies if name in CORE_CRATES}
        for dependency in internal:
            specification = dependencies[dependency]
            if specification != {"workspace": True}:
                raise AssertionError(
                    f"{crate} must inherit {dependency} from workspace.dependencies"
                )
            if dependency not in declared:
                raise AssertionError(
                    f"{dependency} is absent from root workspace.dependencies"
                )
        graph[crate] = internal
    return graph


def changed_files(arguments: list[str]) -> list[str]:
    if arguments:
        return sorted(set(arguments))
    result = subprocess.run(
        ["git", "diff", "--name-only", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return sorted(line for line in result.stdout.splitlines() if line)


def changed_crates(paths: list[str]) -> set[str]:
    crates = set()
    for path in paths:
        parts = Path(path).parts
        if len(parts) >= 2 and parts[0] == "crates" and parts[1] in CORE_CRATES:
            crates.add(parts[1])
    return crates


def downstream_closure(graph: dict[str, set[str]], seeds: set[str]) -> set[str]:
    reverse_dependencies: dict[str, set[str]] = {}
    for crate, dependencies in graph.items():
        for dependency in dependencies:
            reverse_dependencies.setdefault(dependency, set()).add(crate)

    impacted = set(seeds)
    pending = deque(sorted(seeds))
    while pending:
        dependency = pending.popleft()
        for crate in sorted(reverse_dependencies.get(dependency, ())):
            if crate not in impacted:
                impacted.add(crate)
                pending.append(crate)
    return impacted


def validate_proof_obligations(impacted: set[str], lean_source: str) -> list[str]:
    obligations = sorted(
        set().union(*(PROOF_OBLIGATIONS.get(crate, set()) for crate in impacted))
    )
    missing = [name for name in obligations if f"theorem {name}" not in lean_source]
    if missing:
        raise AssertionError(f"Lean proof obligations are missing: {missing}")
    return obligations


def counterexample_replay_source(path: Path = COUNTEREXAMPLE_FIXTURE) -> str:
    fixture = json.loads(path.read_text(encoding="ascii"))
    if fixture.get("schema") != "mrr.counterexample-receipt.v1":
        raise AssertionError("counterexample fixture schema mismatch")
    scalar_fields = ("initialState", "terminalState")
    if any(type(fixture.get(field)) is not int for field in scalar_fields):
        raise AssertionError("counterexample state identities must be integers")
    if fixture.get("initialValid") is not True:
        raise AssertionError("counterexample initial state is not valid")
    if fixture.get("terminalViolates") is not True:
        raise AssertionError("counterexample terminal state does not violate an invariant")
    steps = fixture.get("steps")
    if not isinstance(steps, list):
        raise AssertionError("counterexample steps must be a list")
    rendered_steps = []
    expected_from = fixture["initialState"]
    for step in steps:
        if not isinstance(step, dict):
            raise AssertionError("counterexample step must be an object")
        for field in ("action", "fromState", "toState"):
            if type(step.get(field)) is not int:
                raise AssertionError(f"counterexample step {field} must be an integer")
        if step.get("legal") is not True or step["fromState"] != expected_from:
            raise AssertionError("counterexample transition is not a legal contiguous step")
        expected_from = step["toState"]
        rendered_steps.append(
            "{ action := { digest := Digest256.ofFixtureAtom %d }, "
            "fromState := { digest := Digest256.ofFixtureAtom %d }, "
            "toState := { digest := Digest256.ofFixtureAtom %d }, legal := true }"
            % (step["action"], step["fromState"], step["toState"])
        )
    if expected_from != fixture["terminalState"]:
        raise AssertionError("counterexample terminal state does not match its final step")
    return """
def replayCounterexample : CounterexampleReceipt where
  initialState := { digest := Digest256.ofFixtureAtom %d }
  steps := [%s]
  terminalState := { digest := Digest256.ofFixtureAtom %d }
  initialValid := true
  terminalViolates := true

theorem replay_counterexample_fixture_valid :
    CounterexampleValid replayCounterexample := by
  exact And.intro rfl (And.intro rfl rfl)
""" % (
        fixture["initialState"],
        ", ".join(rendered_steps),
        fixture["terminalState"],
    )


def local_lean_check(
    lean_source: str, toolchain: str = LOCAL_LEAN_TOOLCHAIN
) -> dict[str, object]:
    """Kernel-check exact generated source with a pinned local Lean toolchain."""
    elan = shutil.which("elan")
    if elan is None:
        raise AssertionError("local Lean validation requires elan")

    version = subprocess.run(
        [elan, "run", toolchain, "lean", "--version"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        text=True,
        timeout=30,
    )
    version_text = version.stdout.strip()
    if version.returncode != 0 or LOCAL_LEAN_VERSION not in version_text:
        detail = version.stderr.strip() or version_text or f"exit {version.returncode}"
        raise AssertionError(f"unexpected local Lean toolchain: {detail[-2_000:]}")

    source_sha256 = hashlib.sha256(lean_source.encode()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="mrr-lean-") as temporary_directory:
        source_path = Path(temporary_directory) / "BundleAdmission.generated.lean"
        source_path.write_text(lean_source, encoding="utf-8")
        result = subprocess.run(
            [elan, "run", toolchain, "lean", source_path],
            cwd=ROOT,
            capture_output=True,
            check=False,
            text=True,
            timeout=60,
        )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise AssertionError(
            f"local Lean kernel rejected generated source: {detail[-2_000:]}"
        )
    return {
        "schema": "mrr.local-lean-kernel-receipt.v1",
        "toolchain": toolchain,
        "leanVersion": version_text,
        "sourceSha256": source_sha256,
        "kernelExitStatus": result.returncode,
        "transport": "local-process",
        "temporarySource": "removed",
    }


def local_validation(paths: list[str]) -> tuple[dict, str]:
    graph = dependency_graph()
    paths = changed_files(paths)
    seeds = changed_crates(paths)
    impacted = downstream_closure(graph, seeds)
    lean_source = (ROOT / "proofs/MRRProof/BundleAdmission.lean").read_text()
    lean_source = lean_source.replace(
        "end MRRProof", counterexample_replay_source() + "\nend MRRProof"
    )
    obligations = validate_proof_obligations(impacted, lean_source)
    babel_receipt = validate_org_babel_contract(ROOT)
    return (
        {
            "schema": "mrr.proof-impact-receipt.v1",
            "state": "passed",
            "changedPaths": paths,
            "seedCrates": sorted(seeds),
            "impactedCrates": sorted(impacted),
            "leanObligations": obligations,
            "workspaceDependencies": "validated",
            "orgBabelAuthority": "validated",
            "orgBabel": babel_receipt,
            "counterexampleFixture": "validated-and-replayed",
        },
        lean_source,
    )


def run(arguments: argparse.Namespace) -> dict:
    receipt, lean_source = local_validation(arguments.paths)
    receipt.update(
        {
            "schema": "mrr.local-lean-impact-receipt.v1",
            "localLean": local_lean_check(lean_source, arguments.toolchain),
        }
    )
    return receipt


def impact_main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", help="changed repository-relative paths")
    arguments = parser.parse_args()
    receipt, _ = local_validation(arguments.paths)
    print(json.dumps(receipt, indent=2, sort_keys=True))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="*", help="changed repository-relative paths")
    parser.add_argument("--toolchain", default=LOCAL_LEAN_TOOLCHAIN)
    arguments = parser.parse_args()
    print(json.dumps(run(arguments), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
