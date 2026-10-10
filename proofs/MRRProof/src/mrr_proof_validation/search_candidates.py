"""Schema and process glue for Lean owner/candidate correspondence checks."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from .process_runner import run


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--executions', type=Path, required=True)
    parser.add_argument('--candidates', type=Path, required=True)
    parser.add_argument('--schema', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    project = Path(__file__).resolve().parents[4] / 'proofs/MRRProof/SearchComposition'
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    entries = json.loads(args.candidates.read_text())
    schema = json.loads(args.schema.read_text())
    Draft202012Validator.check_schema(schema)
    for entry in entries:
        Draft202012Validator(schema).validate(entry)
    if not entries:
        raise SystemExit('empty actual candidate evidence')
    print(f'SEARCH-CANDIDATE-SCHEMA-OK: {len(entries)} witnesses', flush=True)

    def check(path: Path, name: str, rejection: str | None = None) -> dict:
        result = run(['lake', 'env', 'lean', '--run', 'CandidateChecks.lean',
                      str(args.executions.resolve()), str(path.resolve())], cwd=project,
                     log=output / f'{name}.log', label='SEARCH-CANDIDATE')
        text = result.output.decode()
        if rejection is None:
            if result.status != 0 or text.count('SEARCH-CANDIDATE-LEAN-OK:') != len(entries):
                raise SystemExit(f'actual candidate correspondence failed: {name}')
        elif result.status == 0 or rejection not in text:
            raise SystemExit(f'expected candidate rejection missing: {name}')
        print(f'SEARCH-CANDIDATE-CONTROL-OK: {name} exit={result.status}', flush=True)
        return {'name': name, 'exit': result.status, 'expectedRejection': rejection}

    checks = [check(args.candidates, 'actual-candidates')]
    controls = [('stale-generation', 'exact execution receipt binding'),
                ('foreign-source', 'exact execution receipt binding'),
                ('wrong-owner', 'missing branch owner'),
                ('forged-candidate', 'exact owner candidate observation correspondence'),
                ('duplicate-candidate', 'unique owner candidate inventory'),
                ('extra-owner', 'exact physical owner union'),
                ('omitted-evidence', 'retained candidate schema')]
    for name, rejection in controls:
        changed = copy.deepcopy(entries)
        entry = changed[-1]
        if name == 'stale-generation':
            entry['compositionReceipt']['generationIdentity'] = 'stale'
        elif name == 'foreign-source':
            entry['compositionReceipt']['sourceDigest'] = 'foreign'
        elif name == 'wrong-owner':
            entry['ownerCandidates'][0][0] = 'foreign.rs'
        elif name == 'forged-candidate':
            entry['ownerCandidates'][0][1] = 'forged'
        elif name == 'duplicate-candidate':
            entry['ownerCandidates'][1][1] = entry['ownerCandidates'][0][1]
        elif name == 'extra-owner':
            entry['ownerCandidates'].append(['extra.rs', 'extra-candidate'])
        elif name == 'omitted-evidence':
            entry['complete'] = False
            entry['ownerCandidates'] = None
        path = output / f'{name}.v1.json'
        path.write_text(json.dumps(changed, indent=2) + '\n')
        checks.append(check(path, name, rejection))
    paths = [args.executions.resolve(), args.candidates.resolve(), args.schema.resolve(),
             project / 'CandidateChecks.lean', Path(__file__).resolve()]
    receipt = {'schemaId': 'mrr.search.candidate-correspondence.qualification', 'schemaVersion': '1',
               'checks': checks, 'sources': [{'path': str(p), 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths],
               'scope': 'Structural correspondence of Data-admitted mapping and actual execution; Rust owns canonical derivation, ASP source authentication is separate'}
    (output / 'candidate-qualification.v1.json').write_text(json.dumps(receipt, indent=2) + '\n')


if __name__ == '__main__':
    main()
