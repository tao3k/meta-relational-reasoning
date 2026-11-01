#!/usr/bin/env python3
"""Acquire the declared Agentic AI Lab source and delegate its owned bootstrap."""

import json
import os
import re
import subprocess
from pathlib import Path


def main() -> None:
    root = Path(__file__).resolve().parents[4]
    pins = re.findall(
        r'"(github\.com/tao3k/agentic-ai-lab)@([0-9a-f]{40})"',
        (root / "gerbil.pkg").read_text(),
    )
    if len(pins) != 1:
        raise ValueError("expected one immutable Agentic AI Lab dependency")
    repository, revision = pins[0]
    package_root = Path(os.environ.get("GERBIL_PATH", root / ".gerbil")) / "pkg"
    target = package_root / repository
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "init", "--quiet", str(target)], check=True)
        subprocess.run(
            [
                "git",
                "-C",
                str(target),
                "remote",
                "add",
                "origin",
                f"https://{repository}.git",
            ],
            check=True,
        )
    current = subprocess.run(
        ["git", "-C", str(target), "rev-parse", "--verify", "HEAD"],
        capture_output=True,
        text=True,
    )
    if current.returncode != 0:
        print(f"Acquire Agentic AI Lab {revision}", flush=True)
        subprocess.run(
            [
                "git",
                "-C",
                str(target),
                "fetch",
                "--no-tags",
                "--depth=1",
                "origin",
                revision,
            ],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(target), "checkout", "--quiet", "--detach", revision],
            check=True,
        )
        target.with_name(target.name + ".tag").write_text(json.dumps(revision) + "\n")
    elif current.stdout.strip() != revision:
        raise ValueError(
            "cached Agentic AI Lab checkout differs from gerbil.pkg; refresh the package cache"
        )
    # Core is Agentic AI Lab's declared gitlink, never a second MRR pin.
    subprocess.run(
        ["git", "-C", str(target), "submodule", "update", "--init", "--depth=1", "--", "core"],
        check=True,
    )
    # These maintained owner recipes install the producer's prerequisites.
    # MRR's subsequent standard deps --install owns the producer package build
    # and its manifest; building the product here would duplicate that work.
    # No compiler flags, link strategy or copied dependency resolver lives here.
    subprocess.run(
        ["just", "--justfile", str(target / "justfile"), "bootstrap-native-poo", "deps"],
        check=True,
    )


if __name__ == "__main__":
    main()
