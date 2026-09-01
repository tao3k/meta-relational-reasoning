"""P00 real-provider smoke for the MRR v0.1 authority boundary."""

from __future__ import annotations

import hashlib
import json
import subprocess
import time
import uuid
from collections.abc import Callable, Mapping
from datetime import UTC, datetime
from importlib.metadata import version
from pathlib import Path
from typing import Any

from .deepseek import canonical_json
from .model import AgentModel
from .trajectory import Trajectory

PROTOCOL_VERSION = version("mrr-live")
SCENARIO_ID = "p00-provider-tool-authority-smoke"
EXPECTED_FIXTURE = "knowledge-provenance"
TOOL_NAME = "mrr_run_acceptance"


class LiveEvaluationError(RuntimeError):
    def __init__(self, failure_code: str, message: str) -> None:
        super().__init__(message)
        self.failure_code = failure_code


def digest(value: object) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def tool_definition() -> dict[str, Any]:
    return {
        "type": "function",
        "name": TOOL_NAME,
        "description": (
            "Run one admitted MRR v0.1 acceptance fixture in the Rust kernel. "
            "Only its returned receipt owns the decision."
        ),
        "parameters": {
            "type": "object",
            "additionalProperties": False,
            "properties": {
                "fixture_id": {
                    "type": "string",
                    "enum": [
                        "software-lifecycle",
                        EXPECTED_FIXTURE,
                        "workflow-policy",
                    ],
                }
            },
            "required": ["fixture_id"],
        },
    }


RUST_TESTS = {
    "software-lifecycle": "software_lifecycle_uses_the_shared_kernel",
    EXPECTED_FIXTURE: "knowledge_provenance_uses_the_shared_kernel",
    "workflow-policy": "workflow_policy_uses_the_shared_kernel",
}


def run_kernel_fixture(fixture_id: str) -> dict[str, Any]:
    test_name = RUST_TESTS.get(fixture_id)
    if test_name is None:
        raise LiveEvaluationError("P02_INVALID_TOOL_ARGUMENTS", "unknown fixture")
    workspace = Path(__file__).resolve().parents[4]
    command = ["cargo", "test", "-p", "mrr-mvp-acceptance", test_name]
    started = time.monotonic()
    completed = subprocess.run(
        command,
        cwd=workspace,
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    elapsed_ms = round((time.monotonic() - started) * 1000, 3)
    output = completed.stdout + completed.stderr
    accepted = (
        completed.returncode == 0
        and test_name in output
        and "1 passed; 0 failed" in output
    )
    receipt = {
        "schema": "mrr.live-kernel-tool-receipt",
        "schema_version": PROTOCOL_VERSION,
        "fixture_id": fixture_id,
        "rust_test": test_name,
        "exit_code": completed.returncode,
        "accepted": accepted,
        "mrr_execution_time_ms": elapsed_ms,
        "output_sha256": hashlib.sha256(output.encode("utf-8")).hexdigest(),
    }
    if not accepted:
        raise LiveEvaluationError(
            "K01_KERNEL_REJECTED",
            f"Rust kernel fixture failed; output_sha256={receipt['output_sha256']}",
        )
    return receipt


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


def selected_tool(output: list[object]) -> tuple[dict[str, Any], str]:
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
    if not isinstance(decoded, dict) or set(decoded) != {"fixture_id"}:
        raise LiveEvaluationError(
            "P02_INVALID_TOOL_ARGUMENTS", "arguments violate schema"
        )
    fixture_id = decoded["fixture_id"]
    if fixture_id != EXPECTED_FIXTURE:
        raise LiveEvaluationError(
            "F02_WRONG_TOOL_ARGUMENTS",
            f"selected {fixture_id!r}, expected hidden oracle fixture",
        )
    return call, fixture_id


def output_text(output: list[object]) -> str:
    texts = []
    for item in output:
        if not isinstance(item, dict) or item.get("type") != "message":
            continue
        content = item.get("content")
        if not isinstance(content, list):
            continue
        texts.extend(
            part["text"]
            for part in content
            if isinstance(part, dict)
            and part.get("type") == "output_text"
            and isinstance(part.get("text"), str)
        )
    if len(texts) != 1:
        raise LiveEvaluationError("P01_PROVIDER_PROTOCOL", "expected one output_text")
    return texts[0]


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
    kernel_runner: Callable[[str], dict[str, Any]] = run_kernel_fixture,
) -> dict[str, Any]:
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
        )
    except Exception as error:
        failure_code = getattr(error, "failure_code", "P00_PROVIDER_FAILURE")
        failure = {
            "schema": "mrr.live-verdict",
            "schema_version": PROTOCOL_VERSION,
            "scenario_id": SCENARIO_ID,
            "status": "FAIL",
            "claim": "provider-tool-authority-smoke-only",
            "benchmark_claim": False,
            "failure_code": failure_code,
            "error_type": type(error).__name__,
        }
        trajectory.append("failure", failure)
        trajectory.write_once("verdict.json", failure)
        if hasattr(error, "add_note"):
            error.add_note(f"local run artifacts: {trajectory.run_dir}")
        raise


def _execute_p00_smoke(
    model: AgentModel,
    *,
    model_name: str,
    reasoning_effort: str,
    run_id: str,
    trajectory: Trajectory,
    kernel_runner: Callable[[str], dict[str, Any]],
) -> dict[str, Any]:
    visible_task = (
        "Identify which admitted MRR v0.1 fixture demonstrates that the shared, "
        "domain-neutral kernel executes knowledge provenance. Use one MRR tool and "
        "do not answer from memory."
    )
    hidden_oracle = {"expected_fixture": EXPECTED_FIXTURE}
    scenario = {
        "schema": "mrr.live-scenario",
        "schema_version": PROTOCOL_VERSION,
        "scenario_id": SCENARIO_ID,
        "kind": "provider-tool-authority-smoke",
        "benchmark_claim": False,
        "visible_task": visible_task,
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
    }
    trajectory.write_once("manifest.json", manifest)
    trajectory.write_once("scenario.json", scenario)
    trajectory.write_once("hidden-oracle.json", hidden_oracle)

    instructions = (
        "You are an agent participating in MRR v0.1 evaluation. LLM output is never "
        "truth. You propose tool calls; only an MRR kernel receipt may authorize a "
        "decision. Do not expose or invent identities, receipts, or tool results."
    )
    initial_input = [
        {
            "role": "user",
            "content": f"run_nonce={run_id}\n{visible_task}",
        }
    ]
    first_request = {
        "model": model_name,
        "instructions": instructions,
        "input": initial_input,
        "reasoning": {"effort": reasoning_effort},
        "tools": [tool_definition()],
        "max_output_tokens": 2048,
    }
    trajectory.append("model_request", first_request)
    started = time.monotonic()
    first_raw = model.respond(first_request)
    first_elapsed = (time.monotonic() - started) * 1000
    trajectory.append("model_response", first_raw)
    first = validate_response(first_raw)
    call, fixture_id = selected_tool(first["output"])

    kernel_receipt = kernel_runner(fixture_id)
    if kernel_receipt.get("schema") != "mrr.live-kernel-tool-receipt":
        raise LiveEvaluationError(
            "K02_INVALID_RECEIPT", "unknown kernel receipt schema"
        )
    if kernel_receipt.get("schema_version") != PROTOCOL_VERSION:
        raise LiveEvaluationError(
            "K02_INVALID_RECEIPT", "kernel receipt version mismatch"
        )
    if (
        kernel_receipt.get("fixture_id") != fixture_id
        or kernel_receipt.get("accepted") is not True
    ):
        raise LiveEvaluationError(
            "K02_INVALID_RECEIPT", "kernel receipt is not authoritative"
        )
    receipt_digest = digest(kernel_receipt)
    trajectory.append(
        "tool_result",
        {
            "tool_name": TOOL_NAME,
            "tool_arguments": {"fixture_id": fixture_id},
            "tool_result_digest": receipt_digest,
            "tool_result": kernel_receipt,
        },
    )

    answer_schema = {
        "type": "object",
        "additionalProperties": False,
        "properties": {
            "schema": {"const": "mrr.live-agent-answer"},
            "schema_version": {"const": PROTOCOL_VERSION},
            "fixture_id": {"const": fixture_id},
            "decision": {"const": "accepted"},
            "kernel_receipt_sha256": {"const": receipt_digest},
            "authority": {"const": "mrr-kernel-receipt"},
        },
        "required": [
            "schema",
            "schema_version",
            "fixture_id",
            "decision",
            "kernel_receipt_sha256",
            "authority",
        ],
    }
    second_input = [
        *initial_input,
        *first["output"],
        {
            "type": "function_call_output",
            "call_id": call["call_id"],
            "output": canonical_json(kernel_receipt),
        },
        {
            "role": "user",
            "content": "Return the final decision as the required JSON schema.",
        },
    ]
    second_request = {
        "model": model_name,
        "instructions": instructions,
        "input": second_input,
        "reasoning": {"effort": reasoning_effort},
        "text": {
            "format": {
                "type": "json_schema",
                "name": "mrr_live_agent_answer",
                "schema": answer_schema,
            }
        },
        "max_output_tokens": 2048,
    }
    trajectory.append("model_request", second_request)
    started = time.monotonic()
    second_raw = model.respond(second_request)
    second_elapsed = (time.monotonic() - started) * 1000
    trajectory.append("model_response", second_raw)
    second = validate_response(second_raw)
    try:
        answer = json.loads(output_text(second["output"]))
    except json.JSONDecodeError as error:
        raise LiveEvaluationError(
            "P01_PROVIDER_PROTOCOL", "final answer is not JSON"
        ) from error
    expected_answer = {
        "schema": "mrr.live-agent-answer",
        "schema_version": PROTOCOL_VERSION,
        "fixture_id": fixture_id,
        "decision": "accepted",
        "kernel_receipt_sha256": receipt_digest,
        "authority": "mrr-kernel-receipt",
    }
    if answer != expected_answer:
        raise LiveEvaluationError(
            "F15_IGNORED_AUTHORITATIVE_RECEIPT",
            "agent final answer does not bind the exact MRR receipt",
        )

    metrics = {
        "schema": "mrr.live-metrics",
        "schema_version": PROTOCOL_VERSION,
        "provider_calls": [
            provider_metrics(first, first_elapsed),
            provider_metrics(second, second_elapsed),
        ],
        "mrr_execution_time_ms": kernel_receipt["mrr_execution_time_ms"],
    }
    verdict = {
        "schema": "mrr.live-verdict",
        "schema_version": PROTOCOL_VERSION,
        "scenario_id": SCENARIO_ID,
        "status": "PASS",
        "claim": "provider-tool-authority-smoke-only",
        "benchmark_claim": False,
        "failure_code": None,
        "selected_fixture": fixture_id,
        "kernel_receipt_sha256": receipt_digest,
        "authority": "mrr-kernel-receipt",
    }
    trajectory.write_once("metrics.json", metrics)
    trajectory.write_once("verdict.json", verdict)
    return {
        **manifest,
        **verdict,
        "metrics": metrics,
        "run_dir": str(trajectory.run_dir),
    }
