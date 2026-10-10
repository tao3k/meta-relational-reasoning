#!/usr/bin/env python3
"""Check the consumer dependency graph for each optional Context feature."""

import subprocess


def graph(features: str) -> str:
    command = [
        "cargo", "tree", "-p", "meta-relational-reasoning", "--locked", "--offline",
        "--edges", "normal", "--prefix", "none",
        "--format", "{p} {f}",
    ]
    if features:
        command.extend(["--no-default-features", "--features", features])
    return subprocess.check_output(command, text=True, timeout=30)


def main() -> None:
    for feature in ["", "agentic-ai-context", "agentic-ai-context-tokens"]:
        output = graph(feature)
        context = [line for line in output.splitlines() if line.startswith("mrr-agentic-ai-context ")]
        if bool(context) != bool(feature):
            raise RuntimeError(f"unexpected Context dependency under {feature or 'default'}")
        if context and ("token-layout" in context[0]) != (feature == "agentic-ai-context-tokens"):
            raise RuntimeError(f"unexpected token feature under {feature}")
        forbidden = ("mrr-data", "tokenizers", "tiktoken", "candle", "reqwest")
        leaked = [line for line in output.splitlines() if line.split()[0].startswith(forbidden)]
        if leaked:
            raise RuntimeError(f"storage/model/network dependency leaked: {leaked}")
        print(f"CONTEXT-FEATURE-OK: {feature or 'default'}", flush=True)


if __name__ == "__main__":
    main()
