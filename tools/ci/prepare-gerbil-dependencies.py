#!/usr/bin/env python3
"""Prefetch the declared POO checkout, then use its native SHA prefetch owner."""

import importlib.util
import json
import re
import subprocess
from pathlib import Path


def main() -> None:
    root = Path(__file__).resolve().parents[2]
    pins = re.findall(
        r'"(github\.com/tao3k/poo-flow)@([0-9a-f]{40})"',
        (root / "gerbil.pkg").read_text(),
    )
    if len(pins) != 1:
        raise ValueError("expected one immutable POO Flow dependency")
    repository, revision = pins[0]
    package_root = root / ".gerbil" / "pkg"
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
        print(f"Prefetch POO Flow {revision}", flush=True)
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
            "cached POO Flow checkout differs from gerbil.pkg; refresh the package cache"
        )
    helper = target / "packages/automation/gerbil_dependency_pin.py"
    spec = importlib.util.spec_from_file_location("poo_native_pins", helper)
    if spec is None or spec.loader is None:
        raise ValueError("declared POO Flow checkout has no native pin helper")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.prepare_native_pins(package_root)


if __name__ == "__main__":
    main()
