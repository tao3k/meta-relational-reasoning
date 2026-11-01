#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 tao3k team and Contributors
# SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later
# Run from MRR's Gerbil environment. The package manager supplies POO and its SDK.
# CI runs prepare-only and --prepared as separate, named steps for diagnosis.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
poo="$(cd "${1:?POO package path required}" && pwd)"
mkdir -p "${2:?output directory required}"
out="$(cd "$2" && pwd)"
mode="${3:-prepare}"
case "$mode" in
    prepare|prepare-only|--prepared) ;;
    *) echo 'expected prepare, prepare-only or --prepared' >&2; exit 2 ;;
esac
cd "$root"
python3 - "$root" "$poo" <<'PY'
import re, subprocess, sys
from pathlib import Path
root, poo = map(Path, sys.argv[1:])
pins = re.findall(r'github.com/tao3k/agentic-ai-lab@([0-9a-f]{40})', (root / 'gerbil.pkg').read_text())
head = subprocess.check_output(['git', '-C', str(poo), 'rev-parse', 'HEAD'], text=True).strip()
if pins != [head]:
    raise ValueError('POO package checkout differs from MRR gerbil.pkg')
print('MRR-POO-DEPENDENCY-OK ' + head, flush=True)
PY
ext=so
if [ "$(uname -s)" = Darwin ]; then ext=dylib; fi
export AGENTIC_AI_LAB_SEMANTIC_LIBRARY="${AGENTIC_AI_LAB_SEMANTIC_LIBRARY:-$out/libagentic_ai_lab_semantic.$ext}"
export AGENTIC_AI_LAB_RUNTIME_V0_LIBRARY="$out/libagentic_ai_lab_runtime_v0.$ext"
export GERBIL_PATH="${GERBIL_PATH:-$root/.gerbil}"
export GERBIL_LOADPATH="$poo/core:$GERBIL_PATH/lib${GERBIL_LOADPATH:+:$GERBIL_LOADPATH}"
if [ "$mode" != --prepared ]; then
    (cd "$poo/core" && gerbil build)
    (cd "$poo" && gerbil build)
    python3 "$poo/bindings/runtime-c/tools/build-semantic.py" --output "$AGENTIC_AI_LAB_SEMANTIC_LIBRARY"
    cargo metadata --locked --format-version 1 --manifest-path runtime/qualification/physical-roundtrip/Cargo.toml \
      | python3 runtime/tools/check_mrr_dependency_graph.py --resolved
    cargo test --locked --manifest-path runtime/Cargo.toml --features mrr-context,orgize-source --no-run
    cargo clippy --locked --manifest-path runtime/Cargo.toml --features mrr-context,data-publication,orgize-source --tests -- -D warnings
    cargo build --locked --manifest-path runtime/qualification/physical-roundtrip/Cargo.toml
    cargo clippy --locked --manifest-path runtime/qualification/physical-roundtrip/Cargo.toml -- -D warnings
    (cd "$poo" && bazelisk build //bindings/runtime-c:runtime_c_shared)
    install -m 0644 "$poo/bazel-bin/bindings/runtime-c/libruntime_c_shared.$ext" "$AGENTIC_AI_LAB_RUNTIME_V0_LIBRARY"
    uv sync --locked --project "$poo/packages/python-runtime" --group dev
    uv build --python "$poo/packages/python-runtime/.venv/bin/python" --project "$poo/packages/python-runtime" --wheel --out-dir "$out/wheels"
    uv pip install --python "$poo/packages/python-runtime/.venv/bin/python" --no-deps --reinstall "$out"/wheels/*.whl
fi
if [ "$mode" = prepare-only ]; then
    echo 'MRR-LIBRARY-PREPARED-OK'
    exit 0
fi
test -s "$AGENTIC_AI_LAB_SEMANTIC_LIBRARY"
test -s "$AGENTIC_AI_LAB_RUNTIME_V0_LIBRARY"
export AGENTIC_AI_LAB_SEMANTIC_SHA256
AGENTIC_AI_LAB_SEMANTIC_SHA256="$(python3 - "$poo" "$AGENTIC_AI_LAB_SEMANTIC_LIBRARY" <<'PY'
import hashlib, sys
from pathlib import Path
poo, library = map(Path, sys.argv[1:])
sys.path.insert(0, str(poo / 'packages/python-runtime/src/agentic_ai_lab_runtime'))
from scheme_wire import loads
manifest = loads(Path(str(library) + '.ss').read_text())
digest = hashlib.sha256(library.read_bytes()).hexdigest()
if digest != manifest['artifactSha256']:
    raise ValueError('native artifact digest mismatch')
for key, source in [('sourceSha256', 'modules/workflow/runtime/semantic.ss'),
                    ('hostSha256', 'bindings/runtime-c/src/semantic_host.c'),
                    ('headerSha256', 'bindings/runtime-c/include/agentic_ai_lab/semantic.h')]:
    if hashlib.sha256((poo / source).read_bytes()).hexdigest() != manifest[key]:
        raise ValueError('native source binding mismatch: ' + source)
print(digest)
PY
)"
export AGENTIC_AI_LAB_RUNTIME_TRACE=1 MRR_NATIVE_PROGRESS=1
export AGENTIC_AI_LAB_RUST_DIFFERENTIAL="$out/python-oracle.ss"
export AGENTIC_AI_LAB_ARCHIVE_DIRECTORY="$out/archive"
export AGENTIC_AI_LAB_MRR_CONTEXT_FLOW_ORACLE="$out/context-flow.ss"
export AGENTIC_AI_LAB_MRR_CLAIM_ORACLE="$out/claims.ss"
export AGENTIC_AI_LAB_MRR_SUPPORT_ORACLE="$out/support.ss"
export AGENTIC_AI_LAB_MRR_FACT_ORACLE="$out/facts.ss"
export AGENTIC_AI_LAB_MRR_RULE_ORACLE="$out/rules.ss"
export AGENTIC_AI_LAB_MRR_DERIVATION_ORACLE="$out/derivations.ss"
export AGENTIC_AI_LAB_NATIVE_DERIVATION_ORACLE="$out/native-proof.ss"
export AGENTIC_AI_LAB_POLICY_ORACLE="$out/policy.ss"
mkdir -p "$AGENTIC_AI_LAB_ARCHIVE_DIRECTORY"
# Preparation above is outside the execution deadline. Every run below retains
# a five-second real-output watchdog and a 45-second wall bound.
bounded() {
    timeout --foreground --signal=TERM --kill-after=1s 45s \
      python3 "$poo/packages/python-runtime/tools/watch.py" "$@"
}
bounded uv run --no-sync --project "$poo/packages/python-runtime" python -c '
import pathlib, sys, agentic_ai_lab_runtime
module = pathlib.Path(agentic_ai_lab_runtime.__file__).resolve()
if not module.is_relative_to(pathlib.Path(sys.prefix).resolve()):
    raise ValueError("installed Python wheel required; source import rejected")
print("MRR-INSTALLED-PYTHON-WHEEL-OK", module, flush=True)
'
bounded uv run --no-sync --project "$poo/packages/python-runtime" python \
  "$poo/packages/python-runtime/tools/differential.py" \
  --library "$AGENTIC_AI_LAB_SEMANTIC_LIBRARY" --output "$AGENTIC_AI_LAB_RUST_DIFFERENTIAL"
bounded cargo test --locked --manifest-path runtime/Cargo.toml --features mrr-context,orgize-source \
  -- --nocapture --test-threads=1
for oracle in support claims context-flow facts rules derivations native-proof policy; do
    test -s "$out/$oracle.ss"
done
bounded uv run --no-sync --project "$poo/packages/python-runtime" pytest -vv -s -o pythonpath= \
  "$poo/packages/python-runtime/tests/unit/test_temporal_support.py" \
  "$poo/packages/python-runtime/tests/unit/test_temporal_policy.py" \
  "$poo/packages/python-runtime/tests/unit/test_temporal_proof.py" \
  "$poo/packages/python-runtime/tests/unit/test_temporal_proof_host.py" \
  "$poo/packages/python-runtime/tests/unit/test_context_restriction.py" \
  "$poo/packages/python-runtime/tests/unit/test_context_use.py"
export AGENTIC_AI_LAB_PHYSICAL_RECEIPT="$out/physical-receipt.ss"
export AGENTIC_AI_LAB_PHYSICAL_ORACLE="$out/physical-oracle.ss"
bounded "$root/runtime/qualification/physical-roundtrip/target/debug/mrr-temporal-physical-roundtrip" \
  | tee "$out/physical.log"
for marker in \
  'Original Scheme v1 transport admitted; unsupported version rejected' \
  'NATIVE-TO-DATA-COMMIT-REPLAY-ACK-LEASE-OK' \
  'NATIVE-TO-DATA-MIXED-SOURCE-FENCE-REFUSED' \
  'NATIVE-TO-DATA-GRANT-ABA-RETIREMENT-ALIAS-REFUSED' \
  'NATIVE-TO-DATA-RETIRED-GRANT-HISTORICAL-REPLAY-RECOVERED' \
  'NATIVE-TO-DATA-STALE-REFUTED-RESTART-OK' \
  'PHYSICAL-CORRECTION -> MRR-REQUERY -> HISTORICAL-STALE -> NATIVE-READMISSION OK' \
  'GQL -> PHYSICAL ROOT -> ORIGINAL MRR SCHEME V1 -> NATIVE TEMPORAL OK'; do
    grep -Fx "$marker" "$out/physical.log" >/dev/null
done
test -s "$AGENTIC_AI_LAB_PHYSICAL_RECEIPT"
test -s "$AGENTIC_AI_LAB_PHYSICAL_ORACLE"
echo 'MRR-LIBRARY-QUALIFICATION-OK'
