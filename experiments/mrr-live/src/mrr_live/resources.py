"""Process adapters for Scheme scheduling and the Rust MRR resource."""

from __future__ import annotations

import hashlib
import subprocess
import time
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any


class ResourceError(RuntimeError):
    """A resource rejected its typed process request or receipt."""


def _workspace() -> Path:
    return Path(__file__).resolve().parents[4]


def run_scheme_driver(arguments: Sequence[str]) -> str:
    """Ask the Scheme AOT scheduler for exactly one resource decision."""
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mrr-gerbil",
            "--bin",
            "mrr-scheme-driver",
            "--",
            *arguments,
        ],
        cwd=_workspace(),
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    value = completed.stdout.strip()
    if completed.returncode != 0 or not value or "\n" in value:
        raise ResourceError("Scheme scheduler rejected the resource transition")
    return value


def _closure_arguments(candidate: Mapping[str, Any]) -> list[str]:
    if set(candidate) != {"source", "target", "edges"}:
        raise ResourceError("closure candidate has unknown or missing fields")
    source = candidate["source"]
    target = candidate["target"]
    edges = candidate["edges"]
    if (
        not isinstance(source, str)
        or not source
        or not isinstance(target, str)
        or not target
    ):
        raise ResourceError("closure endpoints must be non-empty strings")
    if not isinstance(edges, list) or not 1 <= len(edges) <= 64:
        raise ResourceError("closure candidate requires between 1 and 64 edges")
    arguments = [source, target]
    for edge in edges:
        if not isinstance(edge, Mapping) or set(edge) != {"from", "to"}:
            raise ResourceError("each closure edge requires only from and to")
        left = edge["from"]
        right = edge["to"]
        if (
            not isinstance(left, str)
            or not left
            or not isinstance(right, str)
            or not right
        ):
            raise ResourceError("closure edge endpoints must be non-empty strings")
        arguments.extend((left, right))
    return arguments


def _parse_receipt(output: str) -> dict[str, str]:
    rows: dict[str, str] = {}
    for line in output.splitlines():
        key, separator, value = line.partition("=")
        if not separator or not key or key in rows:
            raise ResourceError("malformed or duplicate closure receipt field")
        rows[key] = value
    expected = {"schema", "status", "reachable", "candidate_count", "closure_status"}
    if set(rows) != expected:
        raise ResourceError("closure receipt fields do not match the fixed protocol")
    if (
        rows["schema"] != "mrr.closure-tool-receipt.v1"
        or rows["status"] != "admitted"
        or rows["reachable"] not in {"true", "false"}
        or rows["closure_status"] != "Complete"
    ):
        raise ResourceError("closure receipt is not a complete admission")
    try:
        candidate_count = int(rows["candidate_count"])
    except ValueError as error:
        raise ResourceError("closure candidate count is not an integer") from error
    if candidate_count < 0:
        raise ResourceError("closure candidate count is negative")
    return rows


def run_closure_resource(candidate: Mapping[str, Any]) -> dict[str, Any]:
    """Normalize a model candidate and return an admitted Rust MRR receipt."""
    arguments = _closure_arguments(candidate)
    started = time.monotonic()
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "mrr-conformance",
            "--bin",
            "mrr-closure-tool",
            "--",
            *arguments,
        ],
        cwd=_workspace(),
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )
    elapsed_ms = round((time.monotonic() - started) * 1000, 3)
    if completed.returncode != 0:
        raise ResourceError("Rust MRR closure resource rejected the candidate")
    rows = _parse_receipt(completed.stdout)
    return {
        "schema": "mrr.live-kernel-tool-receipt",
        "status": rows["status"],
        "reachable": rows["reachable"] == "true",
        "candidate_count": int(rows["candidate_count"]),
        "closure_status": rows["closure_status"],
        "mrr_execution_time_ms": elapsed_ms,
        "output_sha256": hashlib.sha256(completed.stdout.encode()).hexdigest(),
    }
