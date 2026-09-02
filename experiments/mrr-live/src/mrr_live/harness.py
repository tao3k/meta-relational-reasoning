"""Natural-task evaluation of the Scheme-driven MRR v0.1 boundary."""

from __future__ import annotations

import hashlib
import json
import time
import uuid
from collections.abc import Callable, Mapping
from datetime import UTC, datetime
from importlib.metadata import version
from pathlib import Path
from typing import Any

from .deepseek import canonical_json
from .model import AgentModel
from .resources import run_closure_resource, run_scheme_driver
from .trajectory import Trajectory

PROTOCOL_VERSION = version("mrr-live")
SCENARIO_ID = "p00-natural-closure-reasoning"
TOOL_NAME = "mrr_derive_closure"


class LiveEvaluationError(RuntimeError):
    """Stable fail-closed evaluation error."""

    def __init__(self, failure_code: str, message: str) -> None:
        super().__init__(message)
        self.failure_code = failure_code


def digest(value: object) -> str:
    return hashlib.sha256(canonical_json(value).encode()).hexdigest()


def tool_definition() -> dict[str, Any]:
    """Provider schema for one non-authoritative graph candidate."""
    return {
        "type": "function",
        "name": TOOL_NAME,
        "description": "Submit a visible directed graph for bounded MRR closure.",
        "parameters": {
            "type": "object",
            "additionalProperties": False,
            "properties": {
                "source": {"type": "string", "minLength": 1},
                "target": {"type": "string", "minLength": 1},
                "edges": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 64,
                    "items": {
                        "type": "object",
                        "additionalProperties": False,
                        "properties": {
                            "from": {"type": "string", "minLength": 1},
                            "to": {"type": "string", "minLength": 1},
                        },
                        "required": ["from", "to"],
                    },
                },
            },
            "required": ["source", "target", "edges"],
        },
    }


def run_kernel_closure(candidate: Mapping[str, Any]) -> dict[str, Any]:
    """Invoke the Rust/Ascent resource without acquiring scheduling authority."""
    receipt = run_closure_resource(candidate)
    return {**receipt, "schema_version": PROTOCOL_VERSION}


def validate_response(response: Mapping[str, Any]) -> dict[str, Any]:
    if response.get("status") != "completed":
        raise LiveEvaluationError("P01_PROVIDER_PROTOCOL", "response not completed")
    response_id = response.get("id")
    model = response.get("model")
    usage = response.get("usage")
    output = response.get("output")
    if not isinstance(response_id, str) or not response_id:
        raise LiveEvaluationError("P01_PROVIDER_PROTOCOL", "missing response id")
    if not isinstance(model, str) or not model:
        raise LiveEvaluationError("P01_PROVIDER_PROTOCOL", "missing resolved model")
    if not isinstance(usage, dict) or not isinstance(output, list):
        raise LiveEvaluationError("P01_PROVIDER_PROTOCOL", "missing output or usage")
    return {"id": response_id, "model": model, "usage": usage, "output": output}


def selected_tool(output: list[object]) -> tuple[dict[str, Any], dict[str, Any]]:
    calls = [
        item
        for item in output
        if isinstance(item, dict) and item.get("type") == "function_call"
    ]
    if len(calls) != 1:
        raise LiveEvaluationError(
            "F01_TOOL_SELECTION", "expected exactly one tool call"
        )
    call = calls[0]
    if call.get("name") != TOOL_NAME or not isinstance(call.get("call_id"), str):
        raise LiveEvaluationError("F01_TOOL_SELECTION", "unknown or unbound tool call")
    arguments = call.get("arguments")
    if not isinstance(arguments, str):
        raise LiveEvaluationError(
            "P02_INVALID_TOOL_ARGUMENTS", "arguments are not JSON"
        )
    try:
        decoded = json.loads(arguments)
    except json.JSONDecodeError as error:
        raise LiveEvaluationError(
            "P02_INVALID_TOOL_ARGUMENTS", "arguments are not valid JSON"
        ) from error
    if not isinstance(decoded, dict):
        raise LiveEvaluationError(
            "P02_INVALID_TOOL_ARGUMENTS", "arguments violate schema"
        )
    return call, decoded


def provider_metrics(response: Mapping[str, Any], elapsed_ms: float) -> dict[str, Any]:
    usage = response["usage"]
    input_details = usage.get("input_tokens_details", {})
    output_details = usage.get("output_tokens_details", {})
    return {
        "response_id": response["id"],
        "resolved_model": response["model"],
        "input_tokens": usage.get("input_tokens"),
        "cached_tokens": input_details.get("cached_tokens"),
        "output_tokens": usage.get("output_tokens"),
        "reasoning_tokens": output_details.get("reasoning_tokens"),
        "total_tokens": usage.get("total_tokens"),
        "wall_time_ms": round(elapsed_ms, 3),
    }


def run_p00_smoke(
    model: AgentModel,
    *,
    model_name: str,
    reasoning_effort: str,
    run_dir: Path,
    kernel_runner: Callable[[Mapping[str, Any]], dict[str, Any]] = run_kernel_closure,
    scheme_runner: Callable[[list[str]], str] = run_scheme_driver,
) -> dict[str, Any]:
    """Run one append-only natural-task evaluation."""
    run_id = f"{datetime.now(UTC).strftime('%Y%m%dT%H%M%SZ')}-{uuid.uuid4().hex[:12]}"
    trajectory = Trajectory(run_dir / run_id)
    try:
        return _execute_p00_smoke(
            model,
            model_name=model_name,
            reasoning_effort=reasoning_effort,
            run_id=run_id,
            trajectory=trajectory,
            kernel_runner=kernel_runner,
            scheme_runner=scheme_runner,
        )
    except Exception as error:
        failure = {
            "schema": "mrr.live-verdict",
            "schema_version": PROTOCOL_VERSION,
            "scenario_id": SCENARIO_ID,
            "status": "FAIL",
            "claim": "natural-task-scheme-mrr-closure",
            "benchmark_claim": False,
            "failure_code": getattr(error, "failure_code", "P00_RESOURCE_FAILURE"),
            "error_type": type(error).__name__,
        }
        trajectory.append("failure", failure)
        trajectory.write_once("verdict.json", failure)
        if hasattr(error, "add_note"):
            error.add_note(f"local run artifacts: {trajectory.run_dir}")
        raise


def _scheme_decision(
    scheme_runner: Callable[[list[str]], str],
    trajectory: Trajectory,
    arguments: list[str],
    expected: str,
) -> None:
    decision = scheme_runner(arguments)
    trajectory.append(
        "scheme_schedule",
        {"arguments": arguments, "decision": decision},
    )
    if decision != expected:
        raise LiveEvaluationError(
            "S01_SCHEME_SCHEDULER", f"expected {expected}, received {decision}"
        )


def _execute_p00_smoke(
    model: AgentModel,
    *,
    model_name: str,
    reasoning_effort: str,
    run_id: str,
    trajectory: Trajectory,
    kernel_runner: Callable[[Mapping[str, Any]], dict[str, Any]],
    scheme_runner: Callable[[list[str]], str],
) -> dict[str, Any]:
    visible_task = (
        "Determine whether artifact-alpha is transitively grounded in "
        "reviewed-source using only the visible directed relations."
    )
    visible_world = {
        "relations": [
            {"from": "artifact-alpha", "to": "build-17"},
            {"from": "build-17", "to": "reviewed-source"},
            {"from": "artifact-beta", "to": "unreviewed-source"},
        ]
    }
    hidden_oracle = {"reachable": True}
    scenario = {
        "schema": "mrr.live-scenario",
        "schema_version": PROTOCOL_VERSION,
        "scenario_id": SCENARIO_ID,
        "kind": "natural-task-visible-world-closure",
        "benchmark_claim": False,
        "visible_task": visible_task,
        "visible_world": visible_world,
        "allowed_tools": [TOOL_NAME],
        "hidden_oracle_sha256": digest(hidden_oracle),
    }
    manifest = {
        "schema": "mrr.live-run-manifest",
        "schema_version": PROTOCOL_VERSION,
        "protocol_version": PROTOCOL_VERSION,
        "run_id": run_id,
        "provider": model.provider,
        "requested_model": model_name,
        "reasoning_effort": reasoning_effort,
        "scenario_id": SCENARIO_ID,
        "cache_policy": "observed-from-provider-usage",
        "artifact_policy": "local-ignored-append-only",
        "scheduler_authority": "gerbil-scheme-aot",
    }
    trajectory.write_once("manifest.json", manifest)
    trajectory.write_once("scenario.json", scenario)
    trajectory.write_once("hidden-oracle.json", hidden_oracle)

    _scheme_decision(
        scheme_runner, trajectory, ["request", "await-proposal"], "model-proposal"
    )
    request = {
        "model": model_name,
        "input": [
            {
                "role": "user",
                "content": canonical_json(
                    {"task": visible_task, "visible_world": visible_world}
                ),
            }
        ],
        "reasoning": {"effort": reasoning_effort},
        "tools": [tool_definition()],
        "max_output_tokens": 2048,
    }
    trajectory.append("model_request", request)
    started = time.monotonic()
    raw_response = model.respond(request)
    elapsed_ms = (time.monotonic() - started) * 1000
    trajectory.append("model_response", raw_response)
    response = validate_response(raw_response)
    call, candidate = selected_tool(response["output"])

    _scheme_decision(
        scheme_runner,
        trajectory,
        ["transition", "await-proposal", "model-proposal", "candidate", "0", "2"],
        "await-closure",
    )
    _scheme_decision(
        scheme_runner, trajectory, ["request", "await-closure"], "mrr-closure"
    )
    kernel_receipt = kernel_runner(candidate)
    if (
        kernel_receipt.get("schema") != "mrr.live-kernel-tool-receipt"
        or kernel_receipt.get("schema_version") != PROTOCOL_VERSION
        or kernel_receipt.get("status") != "admitted"
        or kernel_receipt.get("closure_status") != "Complete"
    ):
        raise LiveEvaluationError("K02_INVALID_RECEIPT", "closure is not authoritative")
    receipt_digest = digest(kernel_receipt)
    trajectory.append(
        "tool_result",
        {
            "tool_name": TOOL_NAME,
            "tool_call_id": call["call_id"],
            "tool_arguments_sha256": digest(candidate),
            "tool_result_digest": receipt_digest,
            "tool_result": kernel_receipt,
        },
    )
    _scheme_decision(
        scheme_runner,
        trajectory,
        ["transition", "await-closure", "mrr-closure", "admitted", "0", "2"],
        "complete",
    )
    if kernel_receipt.get("reachable") is not hidden_oracle["reachable"]:
        raise LiveEvaluationError(
            "F02_WRONG_DECISION", "MRR result misses hidden oracle"
        )

    metrics = {
        "schema": "mrr.live-metrics",
        "schema_version": PROTOCOL_VERSION,
        "provider_calls": [provider_metrics(response, elapsed_ms)],
        "mrr_execution_time_ms": kernel_receipt["mrr_execution_time_ms"],
    }
    verdict = {
        "schema": "mrr.live-verdict",
        "schema_version": PROTOCOL_VERSION,
        "scenario_id": SCENARIO_ID,
        "status": "PASS",
        "claim": "natural-task-scheme-mrr-closure",
        "benchmark_claim": False,
        "failure_code": None,
        "decision": kernel_receipt["reachable"],
        "kernel_receipt_sha256": receipt_digest,
        "authority": "materialized-mrr-closure",
        "scheduler_authority": "gerbil-scheme-aot",
    }
    trajectory.write_once("metrics.json", metrics)
    trajectory.write_once("verdict.json", verdict)
    return {
        **manifest,
        **verdict,
        "metrics": metrics,
        "run_dir": str(trajectory.run_dir),
    }
