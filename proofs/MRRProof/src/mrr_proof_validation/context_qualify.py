#!/usr/bin/env python3
"""Qualify prebuilt Context workflows or tests using the shared output boundary."""

import argparse
import subprocess

from . import native_tests


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=("workflow", "integration", "matrix"))
    args = parser.parse_args()
    if args.target == "matrix":
        targets = [
            (
                "facade-default",
                ["-p", "meta-relational-reasoning", "--lib", "--no-default-features"],
            ),
            ("context-core", ["-p", "mrr-agentic-ai-context", "--no-default-features"]),
            (
                "context-tokens",
                [
                    "-p",
                    "mrr-agentic-ai-context",
                    "--no-default-features",
                    "--features",
                    "token-layout",
                ],
            ),
            (
                "facade-context",
                [
                    "-p",
                    "meta-relational-reasoning",
                    "--lib",
                    "--no-default-features",
                    "--features",
                    "agentic-ai-context",
                ],
            ),
            (
                "facade-tokens",
                [
                    "-p",
                    "meta-relational-reasoning",
                    "--lib",
                    "--no-default-features",
                    "--features",
                    "agentic-ai-context-tokens",
                ],
            ),
            (
                "workflow-integration",
                [
                    "-p",
                    "meta-relational-reasoning",
                    "--test",
                    "context_workflow",
                    "--features",
                    "agentic-ai-context-tokens",
                ],
            ),
        ]
        for name, flags in targets:
            command = ["cargo", "test", *flags, "--locked", "--offline"]
            print(f"CONTEXT-PREPARE: {name}", flush=True)
            status = subprocess.call([*command, "--no-run"])
            if status != 0:
                return status
            print(f"CONTEXT-QUALIFY: {name}", flush=True)
            status = native_tests.qualify([*command, "--", "--nocapture"])
            if status != 0:
                return status
        print("CONTEXT-MATRIX-OK", flush=True)
        return 0
    command = (
        ["target/debug/examples/context_workflow"]
        if args.target == "workflow"
        else [
            "cargo",
            "test",
            "-p",
            "meta-relational-reasoning",
            "--test",
            "context_workflow",
            "--features",
            "agentic-ai-context-tokens",
            "--locked",
            "--offline",
            "--",
            "--nocapture",
        ]
    )
    return native_tests.qualify(command)


if __name__ == "__main__":
    raise SystemExit(main())
