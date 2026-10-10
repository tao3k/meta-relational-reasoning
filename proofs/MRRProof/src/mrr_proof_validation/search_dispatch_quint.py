# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0
"""Finite checker orchestration only; physical Search semantics remain in Rust."""
import argparse
import json
import os
from pathlib import Path
import shutil
import tempfile
from .process_runner import run
from .quint_cases import ROOT, checker_counts, sha256


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--receipt', type=Path, required=True)
    receipt = parser.parse_args().receipt.resolve()
    receipt.parent.mkdir(parents=True, exist_ok=True)
    executable = os.environ.get('MRR_QUINT_BIN') or shutil.which('quint')
    if not executable:
        raise SystemExit('missing pinned Quint executable')
    quint = str(Path(executable).resolve())
    source = ROOT / 'proofs/quint/SearchDispatch.qnt'
    checks = []
    with tempfile.TemporaryDirectory(prefix='dispatch-quint-', dir=receipt.parent) as directory:
        work = Path(directory)
        shutil.copy2(source, work / source.name)
        config = work / 'tlc-config.json'
        config.write_text('{"workers":"1","maxHeap":"-Xmx1G"}\n')
        cases = [("SearchDispatch", "none", None), ("SearchDispatch", "ignoreBudget", "_input = 3"),
                 ("SearchDispatch", "admitLate", "_late = TRUE"),
                 ("SearchDispatch", "ignoreEvidence", "_rejectedPublication = TRUE"),
                 ("SearchCancellation", "none", None), ("SearchCancellation", "cancelAny", "_lostNew = TRUE")]
        cancellation = ROOT / "proofs/quint/SearchCancellation.qnt"
        shutil.copy2(cancellation, work / cancellation.name)
        for model, bug, control in cases:
            case = work / 'Case.qnt'
            case.write_text(f'module Case {{ import {model}(BUG="{bug}").* from "./{model}" }}\n')
            result = run([quint, 'verify', case.name, '--main', 'Case', '--backend', 'tlc',
                          '--apalache-version', '0.62.1', '--server-endpoint', '127.0.0.1:8866',
                          '--invariant', 'safety', '--tlc-config', str(config), '--verbosity', '3'],
                         cwd=work, log=receipt.parent / f'{model}-{bug}.log', label='DISPATCH-QUINT')
            output = result.output.decode()
            generated, distinct, remaining = checker_counts(output)
            if control is None:
                if result.status or remaining or 'No error has been found' not in output:
                    raise SystemExit('dispatch positive finite exhaustion failed')
            elif result.status == 0 or 'Invariant' not in output or control not in output:
                raise SystemExit(f'dispatch negative control missing: {bug}')
            checks.append({'model': model, 'bug': bug, 'exit': result.status, 'generated': generated,
                           'distinct': distinct, 'remaining': remaining, 'logSha256': sha256(result.output)})
            print(f'DISPATCH-QUINT-OK model={model} bug={bug} states={distinct} exit={result.status}', flush=True)
    receipt.write_text(json.dumps({'version': 1, 'modelSha256': sha256(source.read_bytes()),
        'cancellationModelSha256': sha256(cancellation.read_bytes()),
        'scope': 'finite unit reservations; not whole Rust implementation refinement', 'checks': checks}, indent=2) + '\n')


if __name__ == '__main__':
    main()
