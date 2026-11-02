#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0
"""Resolve the POO proof owner and use its immutable LeanPoo dependency gate."""
import argparse
from pathlib import Path
import sys
import shutil
import subprocess
import tomllib
from .producer_dependency import resolve
from .process_runner import run


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('project', type=Path)
    project = parser.parse_args().project.resolve()
    owner = resolve(project)
    result = run([sys.executable, str(owner / 'pin_dependency.py'), str(owner)], cwd=owner,
                 log=project / '.lake/leanpoo-pin.log', label='LEANPOO-PIN')
    if result.status != 0:
        raise SystemExit(f'producer LeanPoo pin failed: exit={result.status}')
    # Lake flattens transitive packages into the consumer's packages directory.
    producer_config = tomllib.loads((owner / 'lakefile.toml').read_text())
    dependencies = [item for item in producer_config['require'] if item['name'] == 'LeanPoo']
    if len(dependencies) != 1:
        raise SystemExit('producer must declare exactly one LeanPoo source')
    dependency = dependencies[0]
    target = project / '.lake/packages/LeanPoo'
    verified = owner / '.lake/packages/LeanPoo'
    if not target.exists():
        shutil.copytree(verified, target, symlinks=True)
    def git(*args: str) -> str:
        return subprocess.check_output(['git', '-C', str(target), *args], text=True, timeout=5).strip()
    if (git('rev-parse', 'HEAD') != dependency['rev']
            or git('remote', 'get-url', 'origin') != dependency['git']
            or git('status', '--porcelain', '--untracked-files=no')):
        raise SystemExit('consumer inherited LeanPoo cache does not match verified producer source')
    print(f"MRR inherited LeanPoo cache verified: {dependency['rev']}", flush=True)


if __name__ == '__main__':
    main()
