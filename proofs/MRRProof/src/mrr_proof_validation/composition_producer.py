"""Qualify the declared POO producer proof dependency before consumer checks."""
import argparse
from pathlib import Path
from .producer_dependency import resolve
from .process_runner import run


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--log', type=Path, required=True)
    args = parser.parse_args()
    owner = resolve(args.project)
    result = run(['bash', '-euc', 'lake build; lake env lean --run CompositionChecks.lean; lake env lean --run SearchTemporalChecks.lean; lake env lean --run SearchReadinessChecks.lean; lake env lean CompositionAxioms.lean'],
                 cwd=owner, log=args.log.resolve(), label='POO-PROOF')
    output = result.output.decode()
    if (result.status != 0 or 'POO-COMPOSITION-OK:' not in output
            or 'POO-COMPOSITION-AXIOM-AUDIT-OK:' not in output
            or 'POO-SEARCH-TEMPORAL-CERTIFICATE-OK' not in output
            or 'POO-SEARCH-READINESS-OK' not in output):
        raise SystemExit(f'POO producer proof failed: exit={result.status}')


if __name__ == '__main__':
    main()
