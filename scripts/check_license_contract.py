#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
#
# SPDX-License-Identifier: AGPL-3.0-only
"""Validate MRR's SPDX, REUSE, and publication license contract."""

from __future__ import annotations

import fnmatch
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EXPRESSION = "AGPL-3.0-only"
COPYRIGHT = "2026 tao3k team and Contributors"
SPDX_LICENSE_LABEL = "SPDX-" + "License-Identifier"
SPDX_COPYRIGHT_LABEL = "SPDX-" + "FileCopyrightText"
LICENSE_TAG = f"{SPDX_LICENSE_LABEL}: {EXPRESSION}"
COPYRIGHT_TAG = f"{SPDX_COPYRIGHT_LABEL}: {COPYRIGHT}"
SIDECAR_FILES = (Path("REUSE.toml"),)
PUBLISHED_LICENSE_COPIES = (
    Path("proofs/MRRProof/LICENSE"),
    Path("experiments/mrr-live/LICENSE"),
)


def git_output(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def submodule_paths() -> tuple[str, ...]:
    result: list[str] = []
    for line in git_output("ls-files", "--stage").splitlines():
        metadata, path = line.split("\t", 1)
        if metadata.split()[0] == "160000":
            result.append(path)
    return tuple(result)


def project_files() -> tuple[Path, ...]:
    submodules = submodule_paths()
    result: list[Path] = []
    for relative in git_output(
        "ls-files", "--cached", "--others", "--exclude-standard"
    ).splitlines():
        if any(relative == root or relative.startswith(root + "/") for root in submodules):
            continue
        path = ROOT / relative
        if path.is_file():
            result.append(path)
    return tuple(result)


def sidecar_patterns(errors: list[str]) -> tuple[str, ...]:
    patterns: list[str] = []
    for sidecar in SIDECAR_FILES:
        path = ROOT / sidecar
        if not path.is_file():
            errors.append(f"{sidecar}: missing REUSE annotation file")
            continue
        payload = tomllib.loads(path.read_text(encoding="utf-8"))
        if payload.get("version") != 1:
            errors.append(f"{sidecar}: version must equal 1")
        base = sidecar.parent.as_posix()
        for index, annotation in enumerate(payload.get("annotations", []), start=1):
            if annotation.get("SPDX-FileCopyrightText") != COPYRIGHT:
                errors.append(f"{sidecar}: annotation {index} has invalid copyright")
            if annotation.get("SPDX-License-Identifier") != EXPRESSION:
                errors.append(f"{sidecar}: annotation {index} has invalid license")
            values: Any = annotation.get("path", [])
            if isinstance(values, str):
                values = [values]
            if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
                errors.append(f"{sidecar}: annotation {index} has invalid paths")
                continue
            for value in values:
                patterns.append(value if base == "." else f"{base}/{value}")
    return tuple(patterns)


def validate_file_coverage(files: tuple[Path, ...], errors: list[str]) -> None:
    patterns = sidecar_patterns(errors)
    for path in files:
        relative = path.relative_to(ROOT).as_posix()
        try:
            header = path.read_text(encoding="utf-8")[:8192]
        except UnicodeDecodeError:
            header = ""
        inline = LICENSE_TAG in header and COPYRIGHT_TAG in header
        annotated = any(fnmatch.fnmatchcase(relative, pattern) for pattern in patterns)
        if not inline and not annotated:
            errors.append(f"{relative}: missing SPDX header or REUSE annotation")


def validate_license_texts(errors: list[str]) -> None:
    canonical_path = ROOT / "LICENSES/AGPL-3.0-only.txt"
    if not canonical_path.is_file():
        errors.append("LICENSES/AGPL-3.0-only.txt: missing canonical license text")
        return
    canonical = canonical_path.read_text(encoding="utf-8")
    if not canonical.startswith(
        "GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3, 19 November 2007\n"
    ):
        errors.append("LICENSES/AGPL-3.0-only.txt: invalid AGPLv3 text")
    combined_path = ROOT / "LICENSE"
    if not combined_path.is_file():
        errors.append("LICENSE: missing repository license")
        return
    combined = combined_path.read_text(encoding="utf-8")
    if LICENSE_TAG not in combined.splitlines()[:3]:
        errors.append("LICENSE: invalid SPDX expression")
    if canonical not in combined:
        errors.append("LICENSE: must contain the verbatim canonical AGPLv3 text")
    for old in ("LICENSES/Apache-2.0.txt", "LICENSES/LGPL-2.1-or-later.txt"):
        if (ROOT / old).exists():
            errors.append(f"{old}: obsolete license text must be removed")


def validate_cargo_metadata(files: tuple[Path, ...], errors: list[str]) -> None:
    for path in files:
        relative = path.relative_to(ROOT)
        scenario_data = "scenarios" in relative.parts and bool(
            {"inputs", "expected"} & set(relative.parts)
        )
        if path.name != "Cargo.toml" or "fixtures" in relative.parts or scenario_data:
            continue
        manifest = tomllib.loads(path.read_text(encoding="utf-8"))
        workspace = manifest.get("workspace")
        if isinstance(workspace, dict):
            workspace_license = workspace.get("package", {}).get("license")
            if workspace_license != EXPRESSION:
                errors.append(
                    f"{relative}: workspace.package.license must equal {EXPRESSION!r}"
                )
        package = manifest.get("package")
        if isinstance(package, dict):
            license_value = package.get("license")
            if license_value not in (EXPRESSION, {"workspace": True}):
                errors.append(
                    f"{relative}: package.license must equal {EXPRESSION!r} "
                    "or inherit from the workspace"
                )


def validate_python_metadata(files: tuple[Path, ...], errors: list[str]) -> None:
    for path in files:
        relative = path.relative_to(ROOT)
        if path.name != "pyproject.toml" or "fixtures" in relative.parts:
            continue
        project = tomllib.loads(path.read_text(encoding="utf-8")).get("project")
        if not isinstance(project, dict):
            continue
        if project.get("license") != EXPRESSION:
            errors.append(f"{relative}: project.license must equal {EXPRESSION!r}")
        if project.get("license-files") != ["LICENSE"]:
            errors.append(f"{relative}: project.license-files must equal ['LICENSE']")


def validate_npm_metadata(files: tuple[Path, ...], errors: list[str]) -> None:
    for path in files:
        relative = path.relative_to(ROOT)
        if path.name != "package.json" or "fixtures" in relative.parts:
            continue
        project = json.loads(path.read_text(encoding="utf-8"))
        if project.get("license") != EXPRESSION:
            errors.append(f"{relative}: license must equal {EXPRESSION!r}")


def validate_published_license_copies(errors: list[str]) -> None:
    canonical = ROOT / "LICENSE"
    if not canonical.is_file():
        return
    expected = canonical.read_bytes()
    for relative in PUBLISHED_LICENSE_COPIES:
        path = ROOT / relative
        if not path.is_file():
            errors.append(f"{relative}: missing published LICENSE copy")
        elif path.read_bytes() != expected:
            errors.append(f"{relative}: must exactly match LICENSE")


def main() -> int:
    errors: list[str] = []
    files = project_files()
    validate_file_coverage(files, errors)
    validate_license_texts(errors)
    validate_cargo_metadata(files, errors)
    validate_python_metadata(files, errors)
    validate_npm_metadata(files, errors)
    validate_published_license_copies(errors)
    if errors:
        for error in errors:
            sys.stderr.write(f"license-contract: {error}\n")
        return 1
    sys.stdout.write(
        f"license-contract: ok ({len(files)} project files, "
        f"{len(submodule_paths())} submodules excluded, {EXPRESSION})\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
