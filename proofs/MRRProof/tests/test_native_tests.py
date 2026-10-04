"""Regression checks for qualification output under CI backpressure."""

import os
import subprocess
import sys
import time
import unittest
from unittest.mock import patch

from mrr_proof_validation import native_tests as native


class OutputRegression(unittest.TestCase):
    def test_short_writes_and_backpressure_preserve_exact_bytes(self):
        payload = b"MODULE x\nCASE-OK y\n\xff\nOK\n"
        forwarded = bytearray()
        calls = 0

        def write(_fd, remaining):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise BlockingIOError()
            count = min(3, len(remaining))
            forwarded.extend(remaining[:count])
            return count

        with patch.object(native.os, "write", side_effect=write):
            with patch.object(native.select, "select", return_value=([], [], [])):
                self.assertTrue(native.forward_output(payload, time.monotonic() + 2))
        self.assertEqual(bytes(forwarded), payload)

    def test_expired_budget_refuses_output(self):
        with patch.object(native.os, "write") as write:
            self.assertFalse(native.forward_output(b"OK\n", time.monotonic() - 1))
        write.assert_not_called()

    def test_qualify_nonblocking_stdout_preserves_failure_status(self):
        read_fd, write_fd = os.pipe()
        os.set_blocking(write_fd, False)
        payload_size = 256 * 1024
        producer = (
            f"import os; os.write(1, b'x' * {payload_size}); raise SystemExit(42)"
        )
        runner = (
            "import sys; import mrr_proof_validation.native_tests as m; "
            f"raise SystemExit(m.qualify([sys.executable, '-c', {producer!r}]))"
        )
        child = subprocess.Popen([sys.executable, "-c", runner], stdout=write_fd)
        os.close(write_fd)
        received = bytearray()
        try:
            with os.fdopen(read_fd, "rb", buffering=0) as stream:
                while chunk := stream.read(4096):
                    received.extend(chunk)
                    time.sleep(0.001)
            self.assertEqual(child.wait(timeout=5), 42)
            self.assertEqual(received, b"x" * payload_size)
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()


if __name__ == "__main__":
    unittest.main()
