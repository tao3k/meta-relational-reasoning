"""CLI for explicit paid live-model evaluation."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import tomllib

from .deepseek import DeepSeekResponseError, DeepSeekResponsesAdapter
from .harness import PROTOCOL_VERSION, LiveEvaluationError, run_p00_smoke


def main() -> int:
    if os.environ.get("MRR_DEEPSEEK_LIVE") != "1":
        print(
            json.dumps(
                {
                    "schema": "mrr.live-skip",
                    "schema_version": PROTOCOL_VERSION,
                    "reason": "set MRR_DEEPSEEK_LIVE=1 to enable the paid live gate",
                },
                sort_keys=True,
            )
        )
        return 0
    project_root = Path(__file__).resolve().parents[2]
    with (project_root / "model-config.toml").open("rb") as source:
        model_config = tomllib.load(source)
    if (
        model_config.get("provider") != "deepseek"
        or not isinstance(model_config.get("model"), str)
        or model_config.get("reasoning_effort") not in {"low", "medium"}
    ):
        print(
            "mrr-live: model-config.toml violates the admitted model profile",
            file=sys.stderr,
        )
        return 1
    model_name = model_config["model"]
    reasoning_effort = model_config["reasoning_effort"]
    run_root = project_root / "runs"
    try:
        receipt = run_p00_smoke(
            DeepSeekResponsesAdapter(os.environ.get("DEEPSEEK_API_KEY", "")),
            model_name=model_name,
            reasoning_effort=reasoning_effort,
            run_dir=run_root,
        )
    except (
        DeepSeekResponseError,
        LiveEvaluationError,
        subprocess.SubprocessError,
    ) as error:
        failure_code = getattr(error, "failure_code", "P00_PROVIDER_FAILURE")
        print(f"mrr-live: {failure_code}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=True, sort_keys=True))
    return 0
