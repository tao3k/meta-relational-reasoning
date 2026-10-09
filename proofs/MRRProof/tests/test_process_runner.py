"""Regression for real CPU progress from short-lived native compilers."""

import select
import subprocess
import sys
import unittest

from mrr_proof_validation import process_runner


class ProcessCpuRegression(unittest.TestCase):
    @unittest.skipUnless(sys.platform.startswith("linux"), "requires Linux /proc")
    def test_reaped_child_cpu_counts_after_short_compiler_invocations(self):
        child = (
            "import time\n"
            "end = time.process_time() + 0.12\n"
            "while time.process_time() < end:\n"
            "    pass\n"
        )
        parent = (
            "import subprocess,sys,time; "
            f"[subprocess.run([sys.executable,'-c',{child!r}],check=True) for _ in range(4)]; "
            "print('done',flush=True); time.sleep(5)"
        )
        process = subprocess.Popen(
            [sys.executable, "-c", parent], stdout=subprocess.PIPE,
            start_new_session=True, text=True,
        )
        try:
            ready, _, _ = select.select([process.stdout], [], [], 3)
            self.assertTrue(ready, "short-lived children did not finish")
            self.assertEqual(process.stdout.readline().strip(), "done")
            cpu = process_runner._owned_cpu(process.pid)
            self.assertGreater(cpu[process.pid], 0.3)
        finally:
            process.terminate()
            process.wait(timeout=5)
            process.stdout.close()
