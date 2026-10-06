"""Preparation receipts must reject changed native executable inputs."""

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from mrr_proof_validation import native_prepare as preparation


class PreparationRegression(unittest.TestCase):
    def test_source_sdk_new_module_and_executable_drift_reject_old_program(self):
        for changed in ("source", "sdk", "new-module", "program"):
            with (
                self.subTest(changed=changed),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory) / "workspace"
                sdk = Path(directory) / "selected-sdk"
                for name in (
                    "gerbil.pkg",
                    "build.ss",
                    "t/sample-test.ss",
                    "proofs/MRRProof/fixtures/native-test-progress.ss",
                    "proofs/MRRProof/src/mrr_proof_validation/native_prepare.py",
                    "proofs/MRRProof/src/mrr_proof_validation/native_tests.py",
                ):
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text("original input")
                for name in (
                    "bin/gxc",
                    "bin/gsc",
                    "include/gambit.h",
                    "lib/static/gerbil__tools__env.scm",
                    "lib/static/gerbil__tools__gxtest.scm",
                ):
                    path = sdk / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text("selected SDK input")
                output = root / ".gerbil/native-qualification"
                output.mkdir(parents=True)
                program = output / "harness"
                program.write_bytes(b"compiled test program")
                with (
                    patch.dict(os.environ, {"GERBIL_HOME": str(sdk)}),
                    patch.object(preparation.Path, "cwd", return_value=root),
                ):
                    receipt = {
                        "snapshot": preparation.snapshot(root),
                        "program": preparation.digest(program),
                    }
                    (output / "receipt.json").write_text(json.dumps(receipt))
                    command, _ = preparation.prepared_command(["t/sample-test.ss"])
                    self.assertEqual(command, [str(program), "t/sample-test.ss"])
                    if changed == "source":
                        (root / "t/sample-test.ss").write_text("new assertion")
                    elif changed == "sdk":
                        (sdk / "lib/static/gerbil__tools__gxtest.scm").write_text(
                            "new SDK"
                        )
                    elif changed == "new-module":
                        (root / "t/another-test.ss").write_text("new test module")
                    else:
                        program.write_bytes(b"different executable")
                    with self.assertRaisesRegex(RuntimeError, "stale"):
                        preparation.prepared_command(["t/sample-test.ss"])


if __name__ == "__main__":
    unittest.main()
