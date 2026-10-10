"""Materialize the declared immutable POO proof source; no search semantics."""
from pathlib import Path
import re
import subprocess
import tomllib
from .process_runner import run


def resolve(project: Path) -> Path:
    project = project.resolve()
    config = tomllib.loads((project / 'lakefile.toml').read_text())
    dependencies = [item for item in config['require'] if item['name'] == 'POOFlowCompositionProofs']
    if len(dependencies) != 1:
        raise SystemExit('expected exactly one POO composition proof owner')
    dependency = dependencies[0]
    if 'path' in dependency:
        owner = (project / dependency['path']).resolve()
        print(f'POO proof owner: {owner} (unpublished local override)', flush=True)
        return owner
    revision = dependency.get('rev', '')
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise SystemExit('POO proof source requires a full immutable SHA')
    subdir = Path(dependency.get('subDir', ''))
    if subdir.is_absolute() or '..' in subdir.parts or not subdir.parts:
        raise SystemExit('POO proof source requires a bounded package subdirectory')
    repository = project / '.lake/packages/POOFlowCompositionProofs'
    repository.mkdir(parents=True, exist_ok=True)

    def git(*args: str) -> str:
        return subprocess.check_output(['git', '-C', str(repository), *args], text=True, timeout=5).strip()

    if not (repository / '.git').exists():
        if any(repository.iterdir()):
            raise SystemExit('refusing to replace a nonempty POO proof dependency')
        git('init', '--quiet')
        git('remote', 'add', 'origin', dependency['git'])
    if git('remote', 'get-url', 'origin') != dependency['git']:
        raise SystemExit('POO proof dependency origin mismatch')
    if git('status', '--porcelain', '--untracked-files=no'):
        raise SystemExit('refusing to replace modified POO proof sources')
    cached = subprocess.run(['git', '-C', str(repository), 'cat-file', '-e', f'{revision}^{{commit}}'],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=5).returncode == 0
    if not cached:
        fetched = run(['git', 'fetch', '--depth=1', 'origin', revision], cwd=repository,
                      log=project / '.lake/poo-proof-fetch.log', label='POO-PROOF-FETCH')
        if fetched.status != 0 or git('rev-parse', 'FETCH_HEAD') != revision:
            raise SystemExit('POO proof dependency fetch failed or returned a foreign revision')
    git('checkout', '--detach', revision)
    if git('rev-parse', 'HEAD') != revision:
        raise SystemExit('POO proof dependency revision mismatch')
    owner = repository / subdir
    owned_config = tomllib.loads((owner / 'lakefile.toml').read_text())
    if owned_config['name'] != 'POOFlowCompositionProofs':
        raise SystemExit('foreign POO proof package at declared source')
    if (owner / 'lean-toolchain').read_bytes() != (project / 'lean-toolchain').read_bytes():
        raise SystemExit('producer and consumer Lean toolchains disagree')
    print(f'POO proof immutable source verified: {revision} package={subdir}', flush=True)
    return owner
