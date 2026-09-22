from __future__ import annotations

import json
from collections.abc import Mapping
from pathlib import Path
from typing import Any

import pytest

from mrr_live.harness import LiveEvaluationError, PROTOCOL_VERSION, run_p00_smoke

CANDIDATE = {
    "source": "artifact-alpha",
    "target": "reviewed-source",
    "edges": [
        {"from": "artifact-alpha", "to": "build-17"},
        {"from": "build-17", "to": "reviewed-source"},
        {"from": "artifact-beta", "to": "unreviewed-source"},
    ],
}


class FakeModel:
    provider = "fake"

    def __init__(self, candidate: Mapping[str, Any] = CANDIDATE) -> None:
        self.candidate = candidate
        self.requests: list[Mapping[str, Any]] = []

    def respond(self, request: Mapping[str, Any]) -> dict[str, Any]:
        self.requests.append(request)
        return {
            "id": "response-1",
            "model": "deepseek-v4-flash",
            "status": "completed",
            "usage": {
                "input_tokens": 20,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens": 10,
                "output_tokens_details": {"reasoning_tokens": 4},
                "total_tokens": 30,
            },
            "output": [
                {
                    "type": "function_call",
                    "name": "mrr_derive_closure",
                    "call_id": "call-1",
                    "arguments": json.dumps(self.candidate),
                }
            ],
        }


class FakeScheme:
    def __init__(self) -> None:
        self.calls: list[list[str]] = []

    def __call__(self, arguments: list[str]) -> str:
        self.calls.append(arguments)
        decisions = {
            ("request", "await-proposal"): "model-proposal",
            (
                "transition",
                "await-proposal",
                "model-proposal",
                "candidate",
                "0",
                "2",
            ): "await-closure",
            ("request", "await-closure"): "mrr-closure",
            (
                "transition",
                "await-closure",
                "mrr-closure",
                "admitted",
                "0",
                "2",
            ): "complete",
        }
        return decisions[tuple(arguments)]


def admitted_receipt(candidate: Mapping[str, Any]) -> dict[str, Any]:
    assert candidate == CANDIDATE
    return {
        "schema": "mrr.live-kernel-tool-receipt",
        "schema_version": PROTOCOL_VERSION,
        "status": "admitted",
        "reachable": True,
        "candidate_count": 3,
        "closure_status": "Complete",
        "mrr_execution_time_ms": 1.25,
        "output_sha256": "a" * 64,
    }


def run(tmp_path: Path, **overrides: Any) -> dict[str, Any]:
    return run_p00_smoke(
        overrides.pop("model", FakeModel()),
        model_name="deepseek-v4-flash",
        reasoning_effort="low",
        run_dir=tmp_path,
        kernel_runner=overrides.pop("kernel_runner", admitted_receipt),
        scheme_runner=overrides.pop("scheme_runner", FakeScheme()),
        **overrides,
    )


def test_natural_task_is_scheduled_by_scheme_and_decided_by_mrr(tmp_path: Path) -> None:
    scheme = FakeScheme()
    model = FakeModel()
    receipt = run(tmp_path, model=model, scheme_runner=scheme)

    assert receipt["status"] == "PASS"
    assert receipt["decision"] is True
    assert receipt["authority"] == "materialized-mrr-closure"
    assert receipt["scheduler_authority"] == "gerbil-scheme-aot"
    assert len(model.requests) == 1
    assert "instructions" not in model.requests[0]
    assert len(scheme.calls) == 4


def test_model_candidate_cannot_replace_kernel_receipt(tmp_path: Path) -> None:
    def forged(_: Mapping[str, Any]) -> dict[str, Any]:
        return {
            "schema": "mrr.live-kernel-tool-receipt",
            "schema_version": PROTOCOL_VERSION,
            "status": "candidate",
            "reachable": True,
            "closure_status": "Complete",
        }

    with pytest.raises(LiveEvaluationError, match="not authoritative"):
        run(tmp_path, kernel_runner=forged)


def test_scheme_scheduler_mismatch_fails_before_model_call(tmp_path: Path) -> None:
    model = FakeModel()

    with pytest.raises(LiveEvaluationError, match="expected model-proposal"):
        run(tmp_path, model=model, scheme_runner=lambda _: "mrr-closure")

    assert model.requests == []


def test_hidden_oracle_scores_kernel_result_not_model_text(tmp_path: Path) -> None:
    def unreachable(candidate: Mapping[str, Any]) -> dict[str, Any]:
        receipt = admitted_receipt(candidate)
        receipt["reachable"] = False
        return receipt

    with pytest.raises(LiveEvaluationError, match="hidden oracle"):
        run(tmp_path, kernel_runner=unreachable)


def test_runs_are_append_only_and_receive_unique_directories(tmp_path: Path) -> None:
    first = run(tmp_path)
    second = run(tmp_path)

    assert first["run_dir"] != second["run_dir"]
    for receipt in (first, second):
        run_dir = Path(receipt["run_dir"])
        assert (run_dir / "manifest.json").is_file()
        assert (run_dir / "scenario.json").is_file()
        assert (run_dir / "hidden-oracle.json").is_file()
        assert (run_dir / "trajectory.jsonl").is_file()
        assert (run_dir / "verdict.json").is_file()
